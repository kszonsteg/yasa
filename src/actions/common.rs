use crate::model::constants::ARENA_WIDTH;
use crate::model::enums::Procedure;
use crate::model::game::GameState;
use crate::model::position::Square;

/// Executes the core logic of moving a player to a position with ball handling
/// and touchdown detection.
///
/// This function:
/// 1. Increments the active player's move counter
/// 2. Updates the player's position
/// 3. Picks up ball if at the target position
/// 4. Updates ball position if carrying
/// 5. Checks for and handles touchdowns
/// 6. Reverts to parent procedure if no touchdown
pub fn execute_player_movement(game_state: &mut GameState, position: Square) -> Result<(), String> {
    let proc = game_state.procedure;
    let parent_proc = game_state.parent_procedure;

    if parent_proc.is_none() {
        return Err(format!(
            "execute_player_movement called without parent_procedure set! Current procedure: {:?}",
            proc
        ));
    }

    let was_carrying = game_state.is_active_player_carrying_ball();
    let active_player = game_state.get_active_player_mut()?;
    let old_moves = active_player.state.moves;
    let ma = active_player.get_ma();
    if old_moves >= ma + 2 {
        return Err(format!(
            "Movement called with invalid state: moves={} already at max (ma+2={}). \
             This indicates a bug in action discovery - available_actions may be stale.",
            old_moves,
            ma + 2
        ));
    }

    active_player.state.moves = active_player.state.moves.checked_add(1).ok_or_else(|| {
        format!(
            "Move counter overflow: player has {} moves (ma={}, procedure: {:?}, parent_procedure: {:?})!",
            old_moves, ma, proc, parent_proc
        )
    })?;
    active_player.position = Some(position);
    if let Some(ref mut active_path) = game_state.active_path {
        active_path.advance();
    }

    if let Ok(ball_position) = game_state.get_ball_position() {
        if ball_position == position {
            game_state.balls[0].is_carried = true;
            // reset path after ball pick up, so the player can move again
            if let Some(ref mut active_path) = game_state.active_path {
                if active_path.is_complete() {
                    game_state.active_path = None;
                }
            }
        }
    }

    if was_carrying || game_state.is_active_player_carrying_ball() {
        game_state.balls[0].position = Some(position);

        let current_team_id = game_state
            .current_team_id
            .as_ref()
            .ok_or("Missing current team id")?;
        let is_home = game_state.is_home_team(current_team_id);
        let is_touchdown = if is_home {
            position.x == 1
        } else {
            position.x == ARENA_WIDTH - 1
        };

        if is_touchdown {
            game_state.procedure = Some(Procedure::Touchdown);

            let team = if is_home {
                game_state.home_team.as_mut()
            } else {
                game_state.away_team.as_mut()
            };

            team.ok_or("Missing team for touchdown")?.score += 1;
        }
    }

    if game_state.procedure != Some(Procedure::Touchdown) {
        game_state.procedure = game_state.parent_procedure;
    }

    Ok(())
}
