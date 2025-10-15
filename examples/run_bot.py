if __name__ == "__main__":
    from typing import cast

    import botbowl
    from yasa.bot import YasaMCTS, YasaRandom

    # Register the bot
    botbowl.register_bot("yasa_random", YasaRandom)
    botbowl.register_bot("yasa_mcts", YasaMCTS)

    # Load configurations, rules, arena and teams
    config = botbowl.load_config("bot-bowl")
    config.competition_mode = False
    config.pathfinding_enabled = False
    ruleset = botbowl.load_rule_set(config.ruleset)
    arena = botbowl.load_arena(config.arena)
    home = botbowl.load_team_by_filename("human", ruleset)
    away = botbowl.load_team_by_filename("human", ruleset)
    config.competition_mode = False
    config.debug_mode = False

    # Play test games
    for i in range(1000):  # Reduced from 1000 for testing
        away_agent = cast(YasaRandom, botbowl.make_bot("yasa_random"))
        home_agent = cast(YasaRandom, botbowl.make_bot("yasa_random"))

        game = botbowl.Game(
            f"{i}",
            home,
            away,
            home_agent,
            away_agent,
            config,
            arena=arena,
            ruleset=ruleset,
        )
        game.config.fast_mode = True

        print(f"Starting game {i + 1}")
        try:
            game.init()
        except Exception as e:
            print(f"Game {i + 1} failed with error: {e}")
            raise

        print("Game is over")
        print(
            f"Game ended with result: Home {game.state.home_team.state.score} - "
            f"Away {game.state.away_team.state.score}"
        )
