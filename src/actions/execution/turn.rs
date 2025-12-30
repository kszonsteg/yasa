use crate::model::action::Action;
use crate::model::enums::Procedure;
use crate::model::game::GameState;

pub fn start_move_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.active_player_id = Some(
        action
            .player()
            .as_ref()
            .ok_or("No player in start move action")?
            .clone(),
    );
    game_state.procedure = Some(Procedure::MoveAction);
    game_state.parent_procedure = Some(Procedure::MoveAction);
    Ok(())
}

pub fn start_blitz_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.active_player_id = Some(
        action
            .player()
            .as_ref()
            .ok_or("No player in start blitz action")?
            .clone(),
    );
    game_state.procedure = Some(Procedure::BlitzAction);
    game_state.parent_procedure = Some(Procedure::BlitzAction);
    game_state
        .turn_state
        .as_mut()
        .ok_or("No Turn state in START_BLITZ action")?
        .blitz_available = false;
    Ok(())
}

pub fn start_pass_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.active_player_id = Some(
        action
            .player()
            .as_ref()
            .ok_or("No player in start pass action")?
            .clone(),
    );
    game_state.procedure = Some(Procedure::PassAction);
    game_state.parent_procedure = Some(Procedure::PassAction);
    game_state
        .turn_state
        .as_mut()
        .ok_or("No Turn state in START_BLITZ action")?
        .pass_available = false;
    Ok(())
}

pub fn start_handoff_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.active_player_id = Some(
        action
            .player()
            .as_ref()
            .ok_or("No player in start handoff action")?
            .clone(),
    );
    game_state.procedure = Some(Procedure::HandoffAction);
    game_state.parent_procedure = Some(Procedure::HandoffAction);
    game_state
        .turn_state
        .as_mut()
        .ok_or("No Turn state in START_BLITZ action")?
        .handoff_available = false;
    Ok(())
}

pub fn start_foul_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.active_player_id = Some(
        action
            .player()
            .as_ref()
            .ok_or("No player in start foul action")?
            .clone(),
    );
    game_state.procedure = Some(Procedure::FoulAction);
    game_state.parent_procedure = Some(Procedure::FoulAction);
    game_state
        .turn_state
        .as_mut()
        .ok_or("No Turn state in START_BLITZ action")?
        .foul_available = false;
    Ok(())
}

pub fn start_block_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.active_player_id = Some(
        action
            .player()
            .as_ref()
            .ok_or("No player in start block action")?
            .clone(),
    );
    game_state.procedure = Some(Procedure::BlockAction);
    game_state.parent_procedure = Some(Procedure::BlockAction);
    Ok(())
}

pub fn end_turn_execution(game_state: &mut GameState) -> Result<(), String> {
    game_state.active_player_id = None;
    game_state.procedure = Some(Procedure::EndTurn);
    game_state.parent_procedure = None;
    Ok(())
}

pub fn end_player_turn_execution(game_state: &mut GameState) -> Result<(), String> {
    let active_player = game_state.get_active_player_mut()?;
    active_player.state.used = true;
    active_player.state.moves = 0;
    active_player.state.squares_moved = vec![];
    active_player.state.has_blocked = false;
    game_state.active_player_id = None;
    game_state.procedure = Some(Procedure::Turn);
    game_state.parent_procedure = None;
    Ok(())
}
