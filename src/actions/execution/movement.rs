use crate::actions::common::execute_player_movement;
use crate::model::action::Action;
use crate::model::enums::Procedure;
use crate::model::game::GameState;

pub fn move_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    let position = action.position().ok_or("Position missing in Move action")?;
    let current_team_id = game_state
        .current_team_id
        .clone()
        .ok_or("Missing current team id")?;

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
        game_state.position = Some(position);
        return Ok(());
    }

    if game_state.get_team_tackle_zones_at(&current_team_id, &position) > 0 {
        game_state.procedure = Some(Procedure::Dodge);
        game_state.position = Some(position);
        return Ok(());
    }

    execute_player_movement(game_state, position)?;

    Ok(())
}

pub fn stand_up_execution(game_state: &mut GameState) -> Result<(), String> {
    let active_player = game_state.get_active_player_mut()?;
    active_player.state.up = true;
    active_player.state.moves += 3;
    Ok(())
}
