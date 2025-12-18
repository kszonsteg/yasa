import json
import math

from botbowl import (
    Action,
    ActionChoice,
    ActionType,
    Game,
    Skill,
    Square,
    Team,
)
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
    # procedure.Reroll,
)


class MCTSDecisionStrategy(DecisionStrategy):
    """MCTS decision strategy."""

    def __init__(self, time_limit: int = 1000, terminal: bool = False):
        super().__init__()
        self.__action_queue: list[Action] = []
        self.terminal = terminal
        self.time_limit = time_limit

    def choose_action(
        self,
        game: Game,
        agent_team: Team,
    ) -> Action:
        """Choose an action using the MCTS algorithm for turns and scripted for others."""
        try:
            if self.__action_queue:
                return self.__action_queue.pop(0)
            proc = game.get_procedure()
            if not isinstance(proc, TURN_PROCEDURES):
                return self.__choose_scripted_action(game, proc, agent_team)
            if self.__is_selecting_block(proc, agent_team):
                return self.__select_block_dice(game.get_available_actions())
            json_state = json.dumps(
                self.serializer.to_json(game.state),
            )
            return self.parser.parse_action(
                json.loads(
                    get_mcts_action(
                        state=json_state,
                        time_limit=self.time_limit,
                        terminal=self.terminal,
                    )
                )["action"],
                game.state,
            )
        except Exception:
            print(
                f"ERROR: MCTS failed to choose action for procedure {game.get_procedure()}"
            )
            with open("error.json", "w") as f:
                json.dump(self.serializer.to_json(game.state), f, indent=4)
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
            return self.__setup(game)
        elif isinstance(proc, procedure.PlaceBall):
            return self.__place_ball(game, agent_team)
        elif isinstance(proc, procedure.HighKick):
            return self.__high_kick(game, agent_team)
        elif isinstance(proc, procedure.Touchback):
            return self.__touchback(game, agent_team)
        elif isinstance(proc, procedure.Reroll):
            return Action(ActionType.USE_REROLL)
        elif isinstance(proc, procedure.Interception):
            return self.__interception(game, agent_team)
        else:
            raise ValueError(f"Unsupported procedure: {proc}")

    def __setup(self, game: Game) -> Action:
        if not self.__action_queue:
            self.__action_queue = []
            actions = [action.action_type for action in game.state.available_actions]
            if ActionType.SETUP_FORMATION_WEDGE in actions:
                self.__action_queue.append(Action(ActionType.SETUP_FORMATION_WEDGE))
            if ActionType.SETUP_FORMATION_ZONE in actions:
                self.__action_queue.append(Action(ActionType.SETUP_FORMATION_ZONE))
            self.__action_queue.append(Action(ActionType.END_SETUP))
        return self.__action_queue.pop(0)

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
    def __high_kick(game, agent_team: Team) -> Action:
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
    def __touchback(game, agent_team: Team) -> Action:
        p = None
        for player in game.get_players_on_pitch(agent_team, up=True):
            if Skill.BLOCK in player.get_skills():
                return Action(ActionType.SELECT_PLAYER, player=player)
            p = player
        return Action(ActionType.SELECT_PLAYER, player=p)

    @staticmethod
    def __interception(game: Game, agent_team: Team) -> Action:
        ball_pos = game.get_ball_position()
        best_dist = float("inf")
        best_action = Action(ActionType.SELECT_NONE)
        for avail in game.state.available_actions:
            if avail.action_type == ActionType.SELECT_PLAYER:
                for player in avail.players:
                    if player.team == agent_team:
                        dist = player.position.distance(ball_pos)
                        if dist < best_dist:
                            best_dist = dist
                            best_action = Action(
                                ActionType.SELECT_PLAYER,
                                player=player,
                                position=ball_pos,
                            )
        return best_action

    @staticmethod
    def __is_selecting_block(proc: procedure.Procedure, agent_team: Team) -> bool:
        return isinstance(proc, procedure.Block) and proc.defender.team == agent_team

    @staticmethod
    def __select_block_dice(available_actions: list[ActionChoice]) -> Action:
        action_types = [action.action_type for action in available_actions]
        if ActionType.SELECT_ATTACKER_DOWN in action_types:
            return Action(ActionType.SELECT_ATTACKER_DOWN)
        elif ActionType.SELECT_BOTH_DOWN in action_types:
            return Action(ActionType.SELECT_BOTH_DOWN)
        elif ActionType.SELECT_PUSH in action_types:
            return Action(ActionType.SELECT_PUSH)
        elif ActionType.SELECT_DEFENDER_STUMBLES in action_types:
            return Action(ActionType.SELECT_DEFENDER_STUMBLES)
        return Action(ActionType.SELECT_DEFENDER_DOWN)
