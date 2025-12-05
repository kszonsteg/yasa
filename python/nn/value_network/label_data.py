import argparse
import json
import math
from pathlib import Path


def exponential_weights(n: int, label: int, k: int = 2) -> list[float]:
    """
    Generate n exponentially increasing weights that end exactly at 1.
    `k` controls how fast the curve approaches 0 at the start.
    """
    return [label * math.exp(k * (i - (n - 1)) / (n - 1)) for i in range(n)]


def label_run(scores: list[float], states: list[Path]) -> list[dict]:
    labeled_states = []
    for score, state in zip(scores, states):
        state = json.loads(state.read_text())
        state["score"] = score
        labeled_states.append(state)
    return labeled_states


def group_game_to_runs(game_dir: Path) -> list[tuple[list[Path], int]]:
    """
    Group game states into runs (drives) and label them.

    A run is a sequence of game states within the same half leading to either:
    - A touchdown (score change) - the TD state is INCLUDED in this run
    - End of half without touchdown

    Run boundaries:
    - Score change: current state is the LAST state of the run (TD was scored)
    - Half change: current state is the FIRST state of a NEW run

    Labels:
    - +1 if home team scored a touchdown in the run
    - -1 if away team scored a touchdown in the run
    - 0 if half ended without touchdown

    Returns:
        List of (files_in_run, label) tuples
    """
    groups = []
    i = 0
    prev_half = None
    prev_score = None
    files_in_current_run = []

    game_file = game_dir / f"{i}.json"
    while game_file.exists():
        data = json.loads(game_file.read_text())
        current_half = data["half"]
        current_score = (data["home_team"]["score"], data["away_team"]["score"])

        # First file in the game
        if prev_half is None:
            prev_half = current_half
            prev_score = current_score
            files_in_current_run.append(game_file)
            i += 1
            game_file = game_dir / f"{i}.json"
            continue

        # Check for boundaries
        half_changed = current_half != prev_half
        home_scored = current_score[0] > prev_score[0]
        away_scored = current_score[1] > prev_score[1]

        if half_changed:
            # Half changed - current state starts a NEW run
            # Complete the previous run with label 0 (no TD in that half)
            if files_in_current_run:
                groups.append((files_in_current_run, 0))

            # Start new run with current file
            files_in_current_run = [game_file]
            prev_half = current_half
            prev_score = current_score
        elif home_scored or away_scored:
            # Score changed - current state is PART of this run (TD was just scored)
            files_in_current_run.append(game_file)

            # Determine label
            label = 1 if home_scored else -1

            # Complete this run
            groups.append((files_in_current_run, label))

            # Start fresh for next run (but don't add current file again)
            files_in_current_run = []
            prev_score = current_score
        else:
            # Continue current run
            files_in_current_run.append(game_file)

        i += 1
        game_file = game_dir / f"{i}.json"

    # Handle the last run (game ended)
    if files_in_current_run:
        groups.append((files_in_current_run, 0))

    return groups


def main(data_dir: str | Path, cleanup: bool = True):
    data_dir = Path(data_dir)
    output_dir = data_dir.parent
    merged_path = output_dir / "merged.jsonl"
    run_counter = 0
    sample_counter = 0

    with merged_path.open("w") as f:
        for process_dir in data_dir.iterdir():
            if not process_dir.is_dir():
                continue
            for game_dir in process_dir.iterdir():
                if not game_dir.is_dir():
                    continue
                print(f"Labeling: {game_dir}")
                runs = group_game_to_runs(game_dir)
                for run in runs:
                    if run[1] == 0 or len(run[0]) == 1:
                        continue
                    scores = exponential_weights(len(run[0]), run[1])
                    labeled_states = label_run(scores, run[0])
                    for state in labeled_states:
                        f.write(json.dumps(state))
                        f.write("\n")
                        sample_counter += 1
                    run_counter += 1

    print(f"Processed {run_counter} runs with {sample_counter} samples")
    print(f"Output written to: {merged_path}")

    # Clean up intermediate game files
    if cleanup:
        import shutil

        print(f"Cleaning up game files in: {data_dir}")
        shutil.rmtree(data_dir)
        print("Cleanup complete.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "-d",
        "--data-dir",
        type=Path,
        default="data/games",
        help="Directory containing game files",
    )
    parser.add_argument(
        "--cleanup",
        action="store_true",
        help="Remove intermediate game files after processing",
    )
    args = parser.parse_args()
    main(args.data_dir, cleanup=args.cleanup)
