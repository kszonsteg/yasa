import json

from botbowl import Action, Game

from yasa.strategies.base import DecisionStrategy
from yasa.yasa_core import get_mcts_action


class MCTSDecisionStrategy(DecisionStrategy):
    """MCTS decision strategy."""

    def choose_action(
        self,
        game: Game,
        time_limit: int,
    ) -> Action:
        """Choose a action using MCTS algorithm."""
        json_state = json.dumps(
            self.serializer.to_json(game.state),
        )
        try:
            return self.parser.parse_action(
                json.loads(get_mcts_action(state=json_state, time_limit=1000))[
                    "action"
                ],
                game.state,
            )
        except ValueError:
            with open("error.json", "w") as f:
                f.write(json_state)
            raise
