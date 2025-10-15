import json
from random import choice

from botbowl import Action, Game

from yasa.components import ActionValidator
from yasa.strategies.base import DecisionStrategy
from yasa.yasa_core import get_actions


class RandomDecisionStrategy(DecisionStrategy):
    """Simple random decision strategy.

    It gets all of the possible actions using rust and validates,
    if the actions from the Rust implementation matches the originial ones.
    """

    def __init__(self):
        super().__init__()
        self.validator = ActionValidator()

    def choose_action(
        self,
        game: Game,
        time_limit: int,
    ) -> Action:
        """Choose a random action from available actions."""
        json_state = json.dumps(
            self.serializer.to_json(game.state),
        )
        try:
            actions = self.parser.parse_actions(
                json.loads(get_actions(json_state))["actions"],
                game.state,
            )
            self.validator.compare_actions(game, actions)
            return choice(actions)
        except ValueError:
            with open("error.json", "w") as f:
                f.write(json_state)
            raise
