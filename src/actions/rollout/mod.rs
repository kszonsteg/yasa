pub mod model;

use crate::model::constants::ARENA_WIDTH;
use crate::model::enums::Procedure;
use crate::model::game::GameState;
use crate::model::position::Square;

pub fn gfi_rollout(game_state: &GameState) -> Result<Vec<model::RolloutOutcome>, String> {
    let mut outcomes = vec![];

    // Successful GFI
    let mut state = game_state.clone();

    let position_vec = state
        .position
        .as_ref()
        .ok_or("Missing position in GFI rollout")?;
    let position = Square {
        x: position_vec[0],
        y: position_vec[1],
    };

    let proc = state.procedure;
    let parent_proc = state.parent_procedure;

    let active_player = state.get_active_player_mut()?;
    let old_moves = active_player.state.moves;
    let ma = active_player.get_ma();

    // Safety check: If moves are already at the maximum, this indicates a bug
    // in action discovery (stale available_actions). Return an error to surface the issue.
    if old_moves >= ma + 2 {
        return Err(format!(
            "GFI rollout called with invalid state: moves={} already at max (ma+2={}). \
             This indicates a bug in action discovery - available_actions may be stale.",
            old_moves,
            ma + 2
        ));
    }

    active_player.state.moves = active_player.state.moves.checked_add(1)
        .ok_or_else(|| format!(
            "Move counter overflow in GFI rollout: player has {} moves (ma={}, procedure: {:?}, parent_procedure: {:?})!",
            old_moves, ma, proc, parent_proc
        ))?;
    active_player.position = Some(position);

    if let Ok(ball_position) = state.get_ball_position() {
        if ball_position == position {
            state.balls[0].is_carried = true;
        }
    }

    if state.is_active_player_carrying_ball() {
        state.balls[0].position = Some(position);

        let is_home = state.is_home_team(
            state
                .current_team_id
                .as_ref()
                .ok_or("GFI rollout cannot check if home team")?,
        );
        let is_touchdown = if is_home {
            position.x == 1
        } else {
            position.x == ARENA_WIDTH - 1
        };

        if is_touchdown {
            state.procedure = Some(Procedure::Touchdown);

            let team = if is_home {
                state.home_team.as_mut()
            } else {
                state.away_team.as_mut()
            };

            team.ok_or("Missing team for touchdown")?.score += 1;
        }
    }

    if state.procedure != Some(Procedure::Touchdown) {
        if let Some(parent_proc) = state.parent_procedure {
            state.procedure = Some(parent_proc);
        } else {
            // If parent_procedure wasn't set, this is an error - GFI should always have a parent
            return Err(format!(
                "GFI rollout: parent_procedure is None! Current procedure: {:?}",
                state.procedure
            ));
        }
    }

    outcomes.push(model::RolloutOutcome::new(5.0 / 6.0, state));

    // Unsuccessful GFI
    let mut state = game_state.clone();

    let position_vec = state
        .position
        .as_ref()
        .ok_or("Missing position in GFI rollout")?;
    let position = Square {
        x: position_vec[0],
        y: position_vec[1],
    };
    let active_player = state.get_active_player_mut()?;
    active_player.position = Some(position);
    active_player.state.up = false;
    state.procedure = Some(Procedure::Turnover);
    // TODO: ball fumble, injuries etc.
    outcomes.push(model::RolloutOutcome::new(1.0 / 6.0, state));

    Ok(outcomes)
}
