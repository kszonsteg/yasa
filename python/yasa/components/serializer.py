from typing import Any

from botbowl import BBDieResult, Dugout, GameState, Player, Team, procedure

PLAYER_ACTIONS = (
    procedure.MoveAction,
    procedure.PassAction,
    procedure.HandoffAction,
    procedure.BlockAction,
    procedure.BlitzAction,
    procedure.FoulAction,
)


class GameStateSerializer:
    """Handles conversion of game state to JSON format."""

    @staticmethod
    def to_json(game_state: GameState) -> dict[str, Any]:
        """Convert game state to JSON-serializable dictionary."""
        basic_info = GameStateSerializer._get_basic_game_info(game_state)
        team_info = GameStateSerializer._get_team_info(game_state)
        procedure_info = GameStateSerializer._get_procedure_info(game_state)
        turn_state = GameStateSerializer._get_turn_state(game_state)

        return {
            **basic_info,
            **team_info,
            **procedure_info,
            **turn_state,
        }

    @staticmethod
    def _get_basic_game_info(game_state: GameState) -> dict[str, Any]:
        """Extract basic game information."""
        return {
            "half": game_state.half,
            "round": game_state.round,
            "game_over": game_state.game_over,
            "weather": game_state.weather.name,
            "balls": [
                {
                    "position": {"x": ball.position.x, "y": ball.position.y}
                    if ball.position
                    else None,
                    "is_carried": ball.is_carried,
                }
                for ball in game_state.pitch.balls
            ],
        }

    @staticmethod
    def _serialize_team(team: Team) -> dict[str, Any]:
        """Serialize team to match Rust Team structure."""
        players_by_id = {}
        player: Player
        for player in team.players:
            if player.position is None:
                continue
            players_by_id[player.player_id] = {
                "player_id": player.player_id,
                "role": player.role.name,
                "skills": [skill.name for skill in player.role.skills],
                "ma": player.role.ma,
                "st": player.role.st,
                "ag": player.role.ag,
                "av": player.role.av,
                "state": {
                    "up": player.state.up,
                    "used": player.state.used,
                    "moves": player.state.moves,
                    "stunned": player.state.stunned,
                    "knocked_out": player.state.knocked_out,
                    "squares_moved": [
                        {"x": pos.x, "y": pos.y} for pos in player.state.squares_moved
                    ],
                    "has_blocked": player.state.has_blocked,
                },
                "position": {"x": player.position.x, "y": player.position.y}
                if player.position
                else None,
            }

        return {
            "team_id": team.team_id,
            "bribes": team.state.bribes,
            "rerolls": team.state.rerolls,
            "score": team.state.score,
            "players_by_id": players_by_id,
        }

    @staticmethod
    def _serialize_dugout(dugout: Dugout) -> dict[str, Any]:
        return {
            "team_id": dugout.team.team_id,
            "reserves": [player.player_id for player in dugout.reserves],
            "kod": [player.player_id for player in dugout.kod],
            "dungeon": [player.player_id for player in dugout.dungeon],
        }

    @staticmethod
    def _get_team_info(game_state: GameState) -> dict[str, Any]:
        """Extract team-related information."""
        return {
            "home_team": GameStateSerializer._serialize_team(game_state.home_team),
            "home_dugout": GameStateSerializer._serialize_dugout(
                game_state.dugouts[game_state.home_team.team_id]
            ),
            "away_team": GameStateSerializer._serialize_team(game_state.away_team),
            "away_dugout": GameStateSerializer._serialize_dugout(
                game_state.dugouts[game_state.away_team.team_id]
            ),
            "kicking_first_half": game_state.kicking_first_half.team_id
            if game_state.kicking_first_half is not None
            else None,
            "receiving_first_half": game_state.receiving_first_half.team_id
            if game_state.receiving_first_half is not None
            else None,
            "kicking_this_drive": game_state.kicking_this_drive.team_id
            if game_state.kicking_this_drive is not None
            else None,
            "receiving_this_drive": game_state.receiving_this_drive.team_id
            if game_state.receiving_this_drive is not None
            else None,
            "coin_toss_winner": game_state.coin_toss_winner.team_id
            if game_state.coin_toss_winner is not None
            else None,
        }

    @staticmethod
    def _serialize_action(action) -> dict[str, Any]:
        """Serialize action to match Rust Action structure."""
        return {
            "action_type": action.action_type.name,
            "player": action.player.player_id if action.player is not None else None,
            "position": {"x": action.position.x, "y": action.position.y}
            if action.position is not None
            else None,
        }

    @staticmethod
    def _get_turn_state(game_state: GameState) -> dict[str, Any]:
        """Extract turn state information."""
        turn = GameStateSerializer.get_latest_turn(game_state)
        if turn:
            return {
                "turn_state": {
                    "blitz": turn.blitz,
                    "quick_snap": turn.quick_snap,
                    "blitz_available": turn.blitz_available,
                    "pass_available": turn.pass_available,
                    "foul_available": turn.foul_available,
                    "handoff_available": turn.handoff_available,
                }
            }
        return {"turn_state": None}

    @staticmethod
    def _get_procedure_info(game_state: GameState) -> dict[str, Any]:
        """Extract procedure-related information."""
        proc = game_state.stack.items[-1] if game_state.stack.items else None
        proc_name = proc.__class__.__name__ if proc else None

        position = None
        if isinstance(proc, procedure.Interception):
            p = game_state.stack.items[-2].position
            position = {"x": p.x, "y": p.y}
        elif isinstance(proc, procedure.FollowUp):
            position = {"x": proc.pos_to.x, "y": proc.pos_to.y}

        result = {
            "procedure": proc_name,
            "parent_procedure": GameStateSerializer.get_parent_procedure_name(
                game_state
            ),
            "current_team_id": game_state.current_team.team_id
            if game_state.current_team is not None
            else None,
            "active_player_id": game_state.active_player.player_id
            if game_state.active_player is not None
            else None,
            "rolls": [
                action.action_type.name for action in game_state.available_actions
            ],
            "position": position,
            "block_context": GameStateSerializer.get_block_context(game_state, proc),
        }
        return result

    @staticmethod
    def get_block_context(
        game_state: GameState, proc: procedure.Procedure
    ) -> dict[str, Any] | None:
        if isinstance(proc, procedure.FollowUp):
            return {
                "attacker": proc.attacker.player_id,
                "defender": proc.defender.player_id,
                "position": {"x": proc.pos_to.x, "y": proc.pos_to.y},
                "knock_out": False,  # doesn't matter
                "push_chain": [],
            }
        elif isinstance(proc, procedure.Block):
            return {
                "attacker": proc.attacker.player_id,
                "defender": proc.defender.player_id,
                "position": {
                    "x": proc.defender.position.x,
                    "y": proc.defender.position.y,
                },
                "knock_out": False,
                "push_chain": [],
            }
        elif isinstance(proc, procedure.Push):
            push_chain = [
                {
                    "attacker": proc.pusher.player_id,
                    "defender": proc.player.player_id,
                    "position": None,
                }
            ]
            i = -2
            parent_proc = game_state.stack.items[i]
            while not isinstance(parent_proc, procedure.Block):
                push_chain.insert(
                    0,
                    {
                        "attacker": parent_proc.pusher.player_id,
                        "defender": parent_proc.player.player_id,
                        "position": {
                            "x": parent_proc.push_to.x,
                            "y": parent_proc.push_to.y,
                        },
                    },
                )
                i -= 1
            return {
                "attacker": parent_proc.attacker.player_id,
                "defender": parent_proc.defender.player_id,
                "position": {
                    "x": parent_proc.defender.position.x,
                    "y": parent_proc.defender.position.y,
                },
                "knock_out": parent_proc.selected_die != BBDieResult.PUSH,
                "push_chain": push_chain,
            }
        return None

    @staticmethod
    def get_latest_turn(game_state: GameState) -> procedure.Turn | None:
        for i in range(len(game_state.stack.items) - 1, -1, -1):
            if isinstance(game_state.stack.items[i], procedure.Turn):
                return game_state.stack.items[i]
        return None

    @staticmethod
    def get_parent_procedure_name(game_state: GameState) -> str | None:
        for i in range(len(game_state.stack.items) - 1, -1, -1):
            if isinstance(game_state.stack.items[i], PLAYER_ACTIONS):
                return game_state.stack.items[i].__class__.__name__
        return None
