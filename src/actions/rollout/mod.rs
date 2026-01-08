pub mod model;

use crate::actions::common::execute_player_movement;
use crate::model::enums::{ActionType, Procedure};
use crate::model::game::GameState;

pub fn gfi_rollout(game_state: &GameState) -> Result<Vec<model::RolloutOutcome>, String> {
    let mut outcomes = vec![];
    let current_team_id = game_state
        .current_team_id
        .clone()
        .ok_or("Missing current team id")?;

    // Successful GFI
    let mut state = game_state.clone();

    let position = state.position.ok_or("Missing position in GFI rollout")?;

    if state.get_team_tackle_zones_at(&current_team_id, &position) > 0 {
        state.procedure = Some(Procedure::Dodge);
        state.position = Some(position);
        outcomes.push(model::RolloutOutcome::new(5.0 / 6.0, state));
    } else {
        execute_player_movement(&mut state, position)?;
        outcomes.push(model::RolloutOutcome::new(5.0 / 6.0, state));
    }

    // Unsuccessful GFI
    let mut state = game_state.clone();

    let position = state.position.ok_or("Missing position in GFI rollout")?;
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
    let position = game_state.position.ok_or("Missing position")?;
    let tz = game_state.get_team_tackle_zones_at(&current_team_id, &position) as i32;

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

    let position = state.position.ok_or("Missing position in dodge rollout")?;
    execute_player_movement(&mut state, position)?;
    outcomes.push(model::RolloutOutcome::new(1.0 - fail_prob, state));

    // Unsuccessful dodge
    let mut state = game_state.clone();

    let position = state.position.ok_or("Missing position in dodge rollout")?;
    let active_player = state.get_active_player_mut()?;
    active_player.position = Some(position);
    active_player.state.up = false;
    state.procedure = Some(Procedure::Turnover);
    state.parent_procedure = None;
    // TODO: ball fumble, injuries etc.
    outcomes.push(model::RolloutOutcome::new(fail_prob, state));

    Ok(outcomes)
}

pub fn block_rollout(game_state: &GameState) -> Result<Vec<model::RolloutOutcome>, String> {
    let mut outcomes = vec![];

    // For now only the probabilites of one dice decided by player
    let mut state = game_state.clone();
    state.procedure = Some(Procedure::Block);
    state.rolls = vec![ActionType::SelectDefenderStumbles];
    outcomes.push(model::RolloutOutcome::new(1.0 / 6.0, state));

    let mut state = game_state.clone();
    state.procedure = Some(Procedure::Block);
    state.rolls = vec![ActionType::SelectDefenderDown];
    outcomes.push(model::RolloutOutcome::new(1.0 / 6.0, state));

    let mut state = game_state.clone();
    state.procedure = Some(Procedure::Block);
    state.rolls = vec![ActionType::SelectPush];
    outcomes.push(model::RolloutOutcome::new(2.0 / 6.0, state));

    let mut state = game_state.clone();
    state.procedure = Some(Procedure::Block);
    state.rolls = vec![ActionType::SelectBothDown];
    outcomes.push(model::RolloutOutcome::new(1.0 / 6.0, state));

    let mut state = game_state.clone();
    state.procedure = Some(Procedure::Block);
    state.rolls = vec![ActionType::SelectAttackerDown];
    outcomes.push(model::RolloutOutcome::new(1.0 / 6.0, state));

    Ok(outcomes)
}
