import logging
from collections.abc import Iterable

from botbowl import Action, ActionType, Game, GameState, Player, Square


class ActionParser:
    """Handles parsing and conversion of actions."""

    @staticmethod
    def parse_action(action: dict, game_state: GameState) -> Action:
        return Action(
            action_type=ActionType[action["action_type"]],
            position=ActionParser._position_to_square(action.get("position")),
            player=ActionParser._player_from_id(game_state, action.get("player")),
        )

    @staticmethod
    def parse_actions(
        action_data: Iterable[dict], game_state: GameState
    ) -> list[Action]:
        """Parse actions from JSON format to botbowl Action objects."""
        return [ActionParser.parse_action(action, game_state) for action in action_data]

    @staticmethod
    def _position_to_square(position: dict[str, int] | None) -> Square | None:
        """Convert position dictionary to Square object."""
        if position is None:
            return None
        return Square(position["x"], position["y"], _out_of_bounds=False)

    @staticmethod
    def _player_from_id(game_state: GameState, player_id: int | None) -> Player | None:
        """Find player by ID in the game state."""
        if player_id is None:
            return None

        for team in [game_state.home_team, game_state.away_team]:
            for player in team.players:
                if player.player_id == player_id:
                    return player

        raise ValueError(f"Player with ID {player_id} not found in the game state.")


class ActionValidator:
    """Validates that actions match between YASA and BotBowl."""

    @staticmethod
    def extract_actions_from_game(game: Game) -> list[Action]:
        """Extract available actions from the game state."""
        actions = []
        for action_choice in game.get_available_actions():
            if action_choice.action_type == ActionType.PLACE_PLAYER:
                continue
            if len(action_choice.players) > 0:
                for player in action_choice.players:
                    actions.append(
                        Action(action_choice.action_type, position=None, player=player)
                    )
            elif len(action_choice.positions) > 0:
                for position in action_choice.positions:
                    actions.append(Action(action_choice.action_type, position=position))
            else:
                actions.append(Action(action_choice.action_type))
        return actions

    @staticmethod
    def compare_actions(game: Game, yasa_actions: list[Action]) -> None:
        """Compare YASA actions with BotBowl actions and log differences."""
        procedure_name = game.state.stack.items[-1].__class__.__name__
        game_actions = ActionValidator.extract_actions_from_game(game)

        logging.info(
            f"Procedure: {procedure_name}, BotBowl actions: {len(game_actions)}, "
            f"Yasa actions: {len(yasa_actions)}"
        )
        logging.debug(f"Game actions: {game_actions}")
        logging.debug(f"Yasa actions: {yasa_actions}")

        # Convert actions to comparable tuples
        def action_to_tuple(action: Action) -> tuple[str, Square | None, Player | None]:
            return action.action_type.name, action.position, action.player

        game_action_tuples = set(action_to_tuple(a) for a in game_actions)
        yasa_action_tuples = set(action_to_tuple(a) for a in yasa_actions)

        only_in_game = game_action_tuples - yasa_action_tuples
        only_in_yasa = yasa_action_tuples - game_action_tuples

        # Log differences
        for action in only_in_game:
            logging.warning(f"Action only in game_actions: {action}")
        for action in only_in_yasa:
            logging.warning(f"Action only in yasa_actions: {action}")

        if only_in_game or only_in_yasa:
            logging.error(f"yasa_actions: {yasa_action_tuples}")
            logging.error(f"game_actions: {game_action_tuples}")
            logging.error(
                f"Procedure: {procedure_name}, BotBowl actions: {len(game_actions)}, "
                f"Yasa actions: {len(yasa_actions)}"
            )
            raise Exception("Mismatch between game_actions and yasa_actions")
