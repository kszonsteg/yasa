import argparse
import json
from pathlib import Path

# Constants matching Rust implementation
ARENA_WIDTH = 28
ARENA_HEIGHT = 17


def evaluate_carrier_endzone_distance(carrier_pos: dict, target_x: int) -> float:
    """
    Evaluate the ball carrier's position relative to the target endzone.
    Returns a score in the range [0.0, 1.0] where:
    - 1.0 = carrier is at the target endzone
    - 0.0 = carrier is at the opposite endzone
    """
    endzone_distance = abs(carrier_pos["x"] - target_x)
    return 0.985 - 0.03 * endzone_distance


def evaluate_player_support(
    player_pos: dict, carrier_pos: dict, max_field_distance: float
) -> float:
    """
    Evaluate how well a player is supporting the ball carrier.
    Returns a score in the range [0.0, 0.1] where:
    - 0.1 = player is at optimal support distance (close to carrier)
    - 0.0 = player is far from the carrier

    Uses Chebyshev distance (max of x,y differences) to match Rust implementation.
    """
    # Chebyshev distance: max(|dx|, |dy|) - matches Rust Square::distance()
    distance_to_carrier = max(
        abs(player_pos["x"] - carrier_pos["x"]), abs(player_pos["y"] - carrier_pos["y"])
    )
    # Closer support is better, but not too close (3-5 squares ideal for support)
    if distance_to_carrier <= 5.0:
        return 0.1 * (1.0 - distance_to_carrier / 5.0)
    else:
        return 0.05 * (1.0 - distance_to_carrier / max_field_distance)


def evaluate_offensive_position(
    team_players: list[dict],
    carrier: dict,
    carrier_pos: dict,
    target_x: int,
    max_field_distance: float,
) -> float:
    """
    Evaluate the team's offensive position when they have the ball.
    Returns a score in the range [0.0, 1.0] where:
    - Higher scores indicate better offensive positioning
    - Includes carrier's proximity to endzone and team support
    """
    carrier_score = evaluate_carrier_endzone_distance(carrier_pos, target_x)
    support_score = 0.0

    for player in team_players:
        if not player.get("position"):
            continue
        if player["player_id"] == carrier["player_id"]:
            continue
        support_score += evaluate_player_support(
            player["position"], carrier_pos, max_field_distance
        )

    # Average support score and add as small bonus (max 0.01)
    num_supporters = len([p for p in team_players if p.get("position")]) - 1
    avg_support = (support_score / num_supporters * 0.01) if num_supporters > 0 else 0.0

    return carrier_score + avg_support


def evaluate_defensive_position(
    team_players: list[dict],
    carrier_pos: dict,
    target_x: int,
    max_field_distance: float,
) -> float:
    """
    Evaluate the team's defensive position when the enemy has the ball.
    Returns a score in the range [-1.0, 0.0] where:
    - Lower scores indicate enemy is closer to our endzone
    - Higher scores (closer to 0) indicate better defensive positioning

    Uses Chebyshev distance to match Rust implementation.
    """
    # Our endzone is at the opposite side of our target endzone
    our_endzone_x = 1 if target_x == ARENA_WIDTH - 1 else ARENA_WIDTH - 1

    enemy_distance_to_our_endzone = abs(carrier_pos["x"] - our_endzone_x)
    # Enemy closeness to our endzone is bad for us (negative score)
    base_defensive_score = -(0.99 - 0.03 * enemy_distance_to_our_endzone)

    team_score = 0.0
    for player in team_players:
        if not player.get("position"):
            continue
        player_pos = player["position"]
        # Chebyshev distance: max(|dx|, |dy|)
        distance_to_carrier = max(
            abs(player_pos["x"] - carrier_pos["x"]),
            abs(player_pos["y"] - carrier_pos["y"]),
        )
        # Being close to an enemy carrier is good for defense
        team_score += 0.4 * (1.0 - distance_to_carrier / max_field_distance)

    # Normalize and add base defensive score
    num_players = len([p for p in team_players if p.get("position")])
    avg_defense = team_score / num_players if num_players > 0 else 0.0
    return base_defensive_score + avg_defense * 0.1


def evaluate_loose_ball_position(
    team_players: list[dict], ball_pos: dict, max_field_distance: float
) -> float:
    """
    Evaluate the team's position when the ball is on the ground.
    Returns a score in the range [0.0, 0.3] where:
    - Higher scores indicate players are closer to the loose ball

    Uses Chebyshev distance to match Rust implementation.
    """
    team_score = 0.0
    for player in team_players:
        if not player.get("position"):
            continue
        player_pos = player["position"]
        # Chebyshev distance: max(|dx|, |dy|)
        distance_to_ball = max(
            abs(player_pos["x"] - ball_pos["x"]), abs(player_pos["y"] - ball_pos["y"])
        )
        # Being close to the ball is good
        team_score += 0.3 * (1.0 - distance_to_ball / max_field_distance)

    # Normalize by number of players
    num_players = len([p for p in team_players if p.get("position")])
    return team_score / num_players if num_players > 0 else 0.0


def evaluate_state_for_team(state: dict, team_id: str, is_home_team: bool) -> float:
    """
    Evaluate a game state from the perspective of a specific team using heuristic.
    Returns a score in the range [-1.0, 1.0] where:
    - 1.0 = definitely winning
    - 0.0 = neutral/even
    - -1.0 = definitely losing
    """
    # Check for touchdown
    if state.get("procedure") == "TOUCHDOWN":
        return 1.0

    # Get ball position and carrier
    ball = state.get("balls", [{}])[0]
    ball_pos = ball.get("position")
    if not ball_pos:
        return 0.0

    ball_carrier_id = ball.get("is_carried_by_player_id")

    # Determine target endzone for this team
    target_x = 1 if is_home_team else ARENA_WIDTH - 1

    # Get team data
    team_data = state.get("home_team") if is_home_team else state.get("away_team")
    if not team_data:
        return 0.0

    team_players = list(team_data.get("players_by_id", {}).values())
    team_players_on_pitch = [p for p in team_players if p.get("position")]

    if not team_players_on_pitch:
        return 0.0

    # Chebyshev distance for max_field_distance: max(ARENA_WIDTH, ARENA_HEIGHT)
    max_field_distance = max(ARENA_WIDTH, ARENA_HEIGHT)

    if ball_carrier_id:
        # Someone carries the ball
        carrier = None
        carrier_team_is_ours = False

        # Find carrier and determine if it's our team
        for player in team_players:
            if player["player_id"] == ball_carrier_id:
                carrier = player
                carrier_team_is_ours = True
                break

        if not carrier:
            # Check opponent's team
            opponent_data = (
                state.get("away_team") if is_home_team else state.get("home_team")
            )
            if opponent_data:
                for player in opponent_data.get("players_by_id", {}).values():
                    if player["player_id"] == ball_carrier_id:
                        carrier = player
                        break

        if not carrier or not carrier.get("position"):
            return 0.0

        carrier_pos = carrier["position"]

        if carrier_team_is_ours:
            # Our team has the ball - evaluate offensive position
            score = evaluate_offensive_position(
                team_players, carrier, carrier_pos, target_x, max_field_distance
            )
        else:
            # Enemy has the ball - evaluate defensive position
            score = evaluate_defensive_position(
                team_players_on_pitch, carrier_pos, target_x, max_field_distance
            )
    else:
        # Ball is on the ground - evaluate loose ball positioning
        score = evaluate_loose_ball_position(
            team_players_on_pitch, ball_pos, max_field_distance
        )

    return max(-1.0, min(1.0, score))


def label_state(state: dict) -> tuple[float, float]:
    """
    Label a state with heuristic evaluation for both teams.
    Returns (home_value, away_value) where each is in [-1, 1].
    """
    home_value = evaluate_state_for_team(state, state["home_team"]["team_id"], True)
    away_value = evaluate_state_for_team(state, state["away_team"]["team_id"], False)
    return home_value, away_value


def label_run(states: list[Path]) -> list[dict]:
    """Label states using heuristic evaluation."""
    labeled_states = []
    for state_path in states:
        state = json.loads(state_path.read_text())
        home_value, away_value = label_state(state)
        state["home_value"] = home_value
        state["away_value"] = away_value
        labeled_states.append(state)
    return labeled_states


def collect_game_states(game_dir: Path) -> list[Path]:
    """
    Collect all game state files from a game directory.
    Returns list of paths to state files.
    """
    states = []
    i = 0
    game_file = game_dir / f"{i}.json"
    while game_file.exists():
        states.append(game_file)
        i += 1
        game_file = game_dir / f"{i}.json"
    return states


def main(data_dir: str | Path, cleanup: bool = True):
    data_dir = Path(data_dir)
    output_dir = data_dir.parent
    merged_path = output_dir / "merged.jsonl"
    game_counter = 0
    sample_counter = 0

    with merged_path.open("w") as f:
        for process_dir in data_dir.iterdir():
            if not process_dir.is_dir():
                continue
            for game_dir in process_dir.iterdir():
                if not game_dir.is_dir():
                    continue
                print(f"Labeling: {game_dir}")
                states = collect_game_states(game_dir)
                if len(states) < 2:
                    # Skip games with too few states
                    continue
                labeled_states = label_run(states)
                for state in labeled_states:
                    f.write(json.dumps(state))
                    f.write("\n")
                    sample_counter += 1
                game_counter += 1

    print(f"Processed {game_counter} games with {sample_counter} samples")
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
