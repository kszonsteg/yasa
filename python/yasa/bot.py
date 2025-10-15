from botbowl import Action, Agent, Game, Team

from yasa.strategies import DecisionStrategy, RandomDecisionStrategy


class YasaBot(Agent):
    """
    Base class for other bots
    """

    def __init__(
        self,
        name: str,
        decision_strategy: DecisionStrategy,
        time_limit: int = 5,
    ):
        super().__init__(name)
        self.__time_limit = time_limit
        self.my_team: Team | None = None
        self.decision_strategy = decision_strategy

    @property
    def time_limit(self) -> int:
        return self.__time_limit

    @time_limit.setter
    def time_limit(self, limit: int):
        if not isinstance(limit, int):
            raise ValueError("Time limit must be an integer")
        self.__time_limit = limit

    def act(self, game: Game) -> Action:
        """Main action method that delegates to the decision strategy."""
        return self.decision_strategy.choose_action(game, self.time_limit)

    def new_game(self, game: Game, team: Team) -> None:
        """Called when a new game starts."""
        self.my_team = team

    def end_game(self, game: Game) -> None:
        """Called when a game ends."""
        pass


class YasaRandom(YasaBot):
    """Random bot with validation."""

    def __init__(self, name: str, time_limit: int = 5):
        super().__init__(name, RandomDecisionStrategy(), time_limit)


if __name__ == "__main__":
    from typing import cast

    import botbowl

    # Register the bot
    botbowl.register_bot("yasa_random", YasaRandom)

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
