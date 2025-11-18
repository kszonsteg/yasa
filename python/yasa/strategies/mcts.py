import json
import math

from botbowl import Action, ActionType, Game, Skill, Square, Team
from botbowl.core import procedure

from yasa.strategies.base import DecisionStrategy
from yasa.yasa_core import get_mcts_action

TURN_PROCEDURES = (
    procedure.Turn,
    procedure.MoveAction,
    procedure.PassAction,
    procedure.HandoffAction,
    procedure.BlockAction,
    procedure.BlitzAction,
    procedure.FoulAction,
    procedure.Block,
    procedure.Push,
    procedure.FollowUp,
    procedure.Reroll,
)


class MCTSDecisionStrategy(DecisionStrategy):
    """MCTS decision strategy."""

    def __init__(self):
        super().__init__()
        self.__setup_actions: list[Action] = []

    def choose_action(
        self,
        game: Game,
        time_limit: int,
        agent_team: Team,
    ) -> Action:
        """Choose an action using the MCTS algorithm for turns and scripted for others."""
        proc = game.get_procedure()
        if not isinstance(proc, TURN_PROCEDURES):
            return self.__choose_scripted_action(game, proc, agent_team)
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

    def __choose_scripted_action(
        self, game: Game, proc: procedure.Procedure, agent_team: Team
    ) -> Action:
        """Choose an action using the scripted strategy."""
        if isinstance(proc, procedure.CoinTossFlip):
            return Action(ActionType.HEADS)
        elif isinstance(proc, procedure.CoinTossKickReceive):
            return Action(ActionType.RECEIVE)
        elif isinstance(proc, procedure.Setup):
            return self.__setup(game, agent_team)
        elif isinstance(proc, procedure.PlaceBall):
            return self.__place_ball(game, agent_team)
        elif isinstance(proc, procedure.HighKick):
            return self.__high_kick(game, agent_team)
        elif isinstance(proc, procedure.Touchback):
            return self.__touchback(game, agent_team)
        else:
            raise ValueError(f"Unsupported procedure: {proc}")

    def __setup(self, game: Game, agent_team: Team) -> Action:
        if not self.__setup_actions:
            self.__setup_actions = [
                Action(ActionType.END_SETUP),
                Action(ActionType.SETUP_FORMATION_WEDGE)
                if game.get_receiving_team() == agent_team
                else Action(ActionType.SETUP_FORMATION_ZONE),
            ]
        return self.__setup_actions.pop(0)

    @staticmethod
    def __place_ball(game: Game, agent_team: Team) -> Action:
        side_width = game.arena.width / 2
        side_height = game.arena.height
        squares_from_left = math.ceil(side_width / 2)
        squares_from_right = math.ceil(side_width / 2)
        squares_from_top = math.floor(side_height / 2)
        left_center = Square(squares_from_left, squares_from_top)
        right_center = Square(
            game.arena.width - 1 - squares_from_right, squares_from_top
        )
        if game.is_team_side(left_center, game.get_opp_team(agent_team)):
            return Action(ActionType.PLACE_BALL, position=left_center)
        return Action(ActionType.PLACE_BALL, position=right_center)

    @staticmethod
    def __high_kick(game, agent_team: Team):
        ball_pos = game.get_ball_position()
        if (
            game.is_team_side(game.get_ball_position(), agent_team)
            and game.get_player_at(game.get_ball_position()) is None
        ):
            for player in game.get_players_on_pitch(agent_team, up=True):
                if (
                    Skill.BLOCK in player.get_skills()
                    and game.num_tackle_zones_in(player) == 0
                ):
                    return Action(
                        ActionType.SELECT_PLAYER, player=player, position=ball_pos
                    )
        return Action(ActionType.SELECT_NONE)

    @staticmethod
    def __touchback(game, agent_team: Team):
        p = None
        for player in game.get_players_on_pitch(agent_team, up=True):
            if Skill.BLOCK in player.get_skills():
                return Action(ActionType.SELECT_PLAYER, player=player)
            p = player
        return Action(ActionType.SELECT_PLAYER, player=p)
