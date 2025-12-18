from typing import cast

from botbowl import Action, Agent, Game, Team

from yasa.strategies import (
    DecisionStrategy,
    MCTSDecisionStrategy,
    RandomDecisionStrategy,
)


class YasaBot(Agent):
    """
    Base class for other bots
    """

    def __init__(
        self,
        name: str,
        decision_strategy: DecisionStrategy,
    ):
        super().__init__(name)
        self.agent_team: Team | None = None
        self.decision_strategy = decision_strategy

    def act(self, game: Game) -> Action:
        """Main action method that delegates to the decision strategy."""
        if self.agent_team is None:
            raise ValueError("Agent team is not set. Did you forget to call new_game?")
        action = self.decision_strategy.choose_action(game, self.agent_team)
        return action

    def new_game(self, game: Game, team: Team) -> None:
        """Called when a new game starts."""
        self.agent_team = team

    def end_game(self, game: Game) -> None:
        """Called when a game ends."""
        pass


class YasaRandom(YasaBot):
    """Random bot with validation."""

    def __init__(self, name: str):
        super().__init__(name, RandomDecisionStrategy())


class YasaMCTS(YasaBot):
    """MCTS bot"""

    def __init__(self, name: str, time_limit: int = 1000, terminal: bool = False):
        super().__init__(name, MCTSDecisionStrategy(time_limit, terminal))
        self.__terminal = terminal
        self.__time_limit = time_limit
        self.decision_strategy = cast(MCTSDecisionStrategy, self.decision_strategy)

    @property
    def time_limit(self) -> int:
        return self.__time_limit

    @time_limit.setter
    def time_limit(self, limit: int):
        if not isinstance(limit, int) or limit <= 0:
            raise ValueError("Time limit must be a positive integer")
        self.__time_limit = limit
        self.decision_strategy.time_limit = limit

    @property
    def terminal(self) -> bool:
        return self.__terminal

    @terminal.setter
    def terminal(self, value: bool):
        if not isinstance(value, bool):
            raise ValueError("Terminal must be a boolean")
        self.__terminal = value
        self.decision_strategy.terminal = value
