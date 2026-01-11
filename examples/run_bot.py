import json
from argparse import ArgumentParser
from pprint import pprint
from typing import cast

import botbowl
from yasa.bot import YasaMCTS, YasaRandom

# Register the bot
botbowl.register_bot("yasa_random", YasaRandom)
botbowl.register_bot("yasa_mcts", YasaMCTS)


def run_game(
    as_home: bool = True,
    game_id: str = "1",
    time_limit: int = 2000,
    terminal: bool = False,
):
    # Load configurations, rules, arena and teams
    config = botbowl.load_config("bot-bowl")
    config.competition_mode = False
    config.pathfinding_enabled = True
    ruleset = botbowl.load_rule_set(config.ruleset)
    arena = botbowl.load_arena(config.arena)
    home = botbowl.load_team_by_filename("human", ruleset)
    away = botbowl.load_team_by_filename("human", ruleset)
    config.competition_mode = False
    config.debug_mode = False

    if as_home:
        home_agent = cast(YasaMCTS, botbowl.make_bot("yasa_mcts"))
        away_agent = cast(YasaRandom, botbowl.make_bot("random"))
        mcts_agent = home_agent
    else:
        home_agent = cast(YasaRandom, botbowl.make_bot("random"))
        away_agent = cast(YasaMCTS, botbowl.make_bot("yasa_mcts"))
        mcts_agent = away_agent
    mcts_agent.time_limit = time_limit
    mcts_agent.terminal = terminal

    game = botbowl.Game(
        game_id,
        home,
        away,
        home_agent,
        away_agent,
        config,
        arena=arena,
        ruleset=ruleset,
    )
    game.config.fast_mode = True

    print(f"Starting game {game_id}, Playing as {'Home' if as_home else 'Away'}")
    try:
        game.init()
    except Exception as e:
        print(f"Game failed with error: {e}")
        print("Procedure Stack:")
        pprint([item.__class__.__name__ for item in game.state.stack.items])
        pprint(game.state.stack.items[-1].__dict__)
        with open("error.json", "w") as f:
            json.dump(game.to_json(), f, indent=4)
        raise
    print(
        f"Game ended with result: Home {game.state.home_team.state.score} - "
        f"Away {game.state.away_team.state.score}"
    )


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument(
        "-n",
        "--num_games",
        type=int,
        default=10,
        help="Number of games to run",
    )
    parser.add_argument(
        "-t",
        "--time_limit",
        type=int,
        default=2000,
        help="Time limit per move in ms",
    )
    parser.add_argument(
        "--terminal",
        action="store_true",
        help="Enable terminal node selection for MCTS bot",
    )
    args = parser.parse_args()
    for i in range(args.num_games):
        run_game(
            game_id=str(i),
            as_home=(i % 2 == 0),
            time_limit=args.time_limit,
            terminal=args.terminal,
        )
