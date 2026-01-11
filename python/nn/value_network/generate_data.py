import argparse
import logging
from pathlib import Path
from typing import cast

import botbowl
from botbowl import Game, register_bot
from bots import MyScriptedBot
from yasa.components import GameStateSerializer


def main(num_games: int, data_dir: str, parallel_index: int):
    """
    Runs a series of games between two Scripted Bot instances to generate training data.
    """
    data_dir = Path(data_dir) / "games" / f"p_{parallel_index}"
    data_dir.mkdir(parents=True, exist_ok=True)

    logging.info(f"Starting data generation for {num_games} games...")
    logging.info(f"Data will be saved in: {data_dir}")
    register_bot("MyScriptedBot", MyScriptedBot)
    config = botbowl.load_config("bot-bowl")
    config.competition_mode = False
    config.pathfinding_enabled = True
    config.debug_mode = False
    ruleset = botbowl.load_rule_set(config.ruleset)
    arena = botbowl.load_arena(config.arena)

    # --- Game Loop ---
    for i in range(num_games):
        logging.info(f"--- Starting Game {i + 1}/{num_games} on {parallel_index}---")

        home_team = botbowl.load_team_by_filename("human", ruleset)
        away_team = botbowl.load_team_by_filename("human", ruleset)

        home_agent = cast(MyScriptedBot, botbowl.make_bot("MyScriptedBot"))
        away_agent = cast(MyScriptedBot, botbowl.make_bot("MyScriptedBot"))

        game = Game(
            str(i),
            home_team,
            away_team,
            home_agent,
            away_agent,
            config,
            arena=arena,
            ruleset=ruleset,
            save_state_path=str(data_dir),
            save_state_serializer=GameStateSerializer,
        )
        game.config.fast_mode = True
        game.config.pathfinding_enabled = True

        try:
            game.init()
            logging.info(f"Game {i + 1} finished successfully.")
            logging.info(
                f"  Result: Home {game.state.home_team.state.score} - Away {game.state.away_team.state.score}"
            )
        except Exception as e:
            logging.warning(f"Game {i + 1} failed with error: {e}")
            # error_log_path = f"generate_data_error_state_{i}.json"
            # with open(error_log_path, "w") as f:
            #     json.dump(GameStateSerializer.to_json(game.state), f)
            # logging.warning(f"  Error state saved to {error_log_path}")
            continue  # Continue to the next game
    logging.info(f"--- Data generation complete on {parallel_index} ---")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "-n",
        "--num-games",
        type=int,
        default=10,
        help="Number of games to generate data for",
    )
    parser.add_argument(
        "-d",
        "--data-dir",
        type=Path,
        default="data",
        help="Directory to store game files",
    )
    parser.add_argument(
        "-p",
        "--parallel-index",
        type=int,
        default=0,
        help="Parallel index to handle running script using parallel index",
    )
    args = parser.parse_args()
    logger = logging.getLogger()
    logger.setLevel(logging.INFO)
    main(args.num_games, args.data_dir, args.parallel_index)
