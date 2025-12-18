use crate::actions::pathfinding::find_optimal_path;
use crate::model::action::Action;
use crate::model::constants::ARENA_WIDTH;
use crate::model::enums::Procedure;
use crate::model::game::GameState;

pub fn move_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    let position = action.position().ok_or("Position missing in Move action")?;
    let current_team_id = game_state
        .current_team_id
        .clone()
        .ok_or("Missing current team id")?;

    let active_player_id = game_state
        .active_player_id
        .clone()
        .ok_or("Missing active player id")?;

    let needs_new_path = {
        let active_player = game_state.get_active_player()?;
        match &active_player.state.active_path {
            Some(path) => {
                // If the target doesn't match, we need a new path
                path.target != position
            }
            None => true,
        }
    };

    if needs_new_path {
        let path = find_optimal_path(game_state, &active_player_id, &position)?;
        let active_player = game_state.get_active_player_mut()?;
        active_player.state.active_path = Some(path);
    }

    let next_position = {
        let active_player = game_state.get_active_player()?;
        let path = active_player
            .state
            .active_path
            .as_ref()
            .ok_or("No active path found")?;
        path.next_position()
            .copied()
            .ok_or("No next position in path")?
    };

    let gfi_required = {
        let active_player = game_state.get_active_player()?;
        let moves = active_player.state.moves;
        let ma = active_player.get_ma();

        moves.checked_add(1).ok_or_else(|| {
            format!(
                "Move counter overflow: player has {} moves (should never exceed ma+2={})!",
                moves,
                ma + 2
            )
        })? > ma
    };

    if gfi_required {
        game_state.parent_procedure = game_state.procedure;
        game_state.procedure = Some(Procedure::GFI);
        game_state.position = Some(vec![next_position.x, next_position.y]);
        return Ok(());
    }

    // Execute the move to the next position in the path
    execute_move_to_position(game_state, next_position, &current_team_id)?;

    // Advance the path
    let active_player = game_state.get_active_player_mut()?;
    if let Some(path) = &mut active_player.state.active_path {
        path.advance();
        // Clear path if we've reached the target
        if path.is_complete() {
            active_player.state.active_path = None;
        }
    }

    Ok(())
}

/// Execute a move to a specific position (shared logic for regular moves and GFI)
fn execute_move_to_position(
    game_state: &mut GameState,
    position: crate::model::position::Square,
    current_team_id: &str,
) -> Result<(), String> {
    let was_carrying = game_state.is_active_player_carrying_ball();
    let proc = game_state.procedure;

    let active_player = game_state.get_active_player_mut()?;
    let old_moves = active_player.state.moves;
    active_player.state.moves = active_player.state.moves.checked_add(1).ok_or_else(|| {
        format!(
            "Move counter overflow when incrementing: player has {} moves! Procedure: {:?}",
            old_moves, proc
        )
    })?;
    active_player.position = Some(position);

    if let Ok(ball_position) = game_state.get_ball_position() {
        if ball_position == position {
            game_state.balls[0].is_carried = true;
        }
    }

    if was_carrying || game_state.is_active_player_carrying_ball() {
        game_state.balls[0].position = Some(position);

        let is_home = game_state.is_home_team(&current_team_id.to_string());
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

    Ok(())
}

pub fn stand_up_execution(game_state: &mut GameState) -> Result<(), String> {
    let active_player = game_state.get_active_player_mut()?;
    active_player.state.up = true;
    active_player.state.moves += 3;
    Ok(())
}
