pub mod model;

use crate::actions::common::execute_player_movement;
use crate::model::enums::Procedure;
use crate::model::game::GameState;
use crate::model::position::Square;

pub fn gfi_rollout(game_state: &GameState) -> Result<Vec<model::RolloutOutcome>, String> {
    let mut outcomes = vec![];
    let current_team_id = game_state
        .current_team_id
        .clone()
        .ok_or("Missing current team id")?;

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

    if state.get_team_tackle_zones_at(&current_team_id, &position) > 0 {
        state.procedure = Some(Procedure::Dodge);
        state.position = Some(vec![position.x, position.y]);
        outcomes.push(model::RolloutOutcome::new(5.0 / 6.0, state));
    } else {
        execute_player_movement(&mut state, position)?;
        outcomes.push(model::RolloutOutcome::new(5.0 / 6.0, state));
    }

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
    state.parent_procedure = None;
    // TODO: ball fumble, injuries etc.
    outcomes.push(model::RolloutOutcome::new(1.0 / 6.0, state));

    Ok(outcomes)
}

pub fn dodge_rollout(game_state: &GameState) -> Result<Vec<model::RolloutOutcome>, String> {
    let mut outcomes = vec![];

    // Calculate probability

    let ag = game_state.get_active_player()?.get_ag();
    // agility table: 1 -> 6+, 2 -> 5+, 3 -> 4+, 4 -> 3+, 5 -> 2+, 6 -> 1+
    let min_required = match ag {
        1 => 6,
        2 => 5,
        3 => 4,
        4 => 3,
        5 => 2,
        _ => 1,
    };

    let current_team_id = game_state
        .current_team_id
        .clone()
        .ok_or("Missing current team id")?;
    let position = game_state.position.as_ref().ok_or("Missing position")?;
    let tz = game_state.get_team_tackle_zones_at(
        &current_team_id,
        &Square {
            x: position[0],
            y: position[1],
        },
    ) as i32;

    // Calculate failure probability
    let mut failures = 1;
    for roll in 2..=5 {
        if (roll + 1 - tz) < min_required {
            failures += 1;
        }
    }
    let fail_prob = failures as f64 / 6.0;

    // Successful dodge
    let mut state = game_state.clone();

    let position_vec = state
        .position
        .as_ref()
        .ok_or("Missing position in dodge rollout")?;
    let position = Square {
        x: position_vec[0],
        y: position_vec[1],
    };

    execute_player_movement(&mut state, position)?;
    outcomes.push(model::RolloutOutcome::new(1.0 - fail_prob, state));

    // Unsuccessful dodge
    let mut state = game_state.clone();

    let position_vec = state
        .position
        .as_ref()
        .ok_or("Missing position in dodge rollout")?;
    let position = Square {
        x: position_vec[0],
        y: position_vec[1],
    };
    let active_player = state.get_active_player_mut()?;
    active_player.position = Some(position);
    active_player.state.up = false;
    state.procedure = Some(Procedure::Turnover);
    state.parent_procedure = None;
    // TODO: ball fumble, injuries etc.
    outcomes.push(model::RolloutOutcome::new(fail_prob, state));

    Ok(outcomes)
}
