from abc import ABC, abstractmethod

from botbowl import Action, Game

from yasa.components import ActionParser, GameStateSerializer


class DecisionStrategy(ABC):
    """Abstract base class for decision-making strategies."""

    def __init__(self):
        self.serializer = GameStateSerializer()
        self.parser = ActionParser()

    @abstractmethod
    def choose_action(
        self,
        game: Game,
        time_limit: int,
    ) -> Action:
        """Choose an action from the available actions."""
        pass
