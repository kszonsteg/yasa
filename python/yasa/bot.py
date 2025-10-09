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
