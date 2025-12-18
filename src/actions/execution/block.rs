use crate::model::action::Action;
use crate::model::enums::Procedure;
use crate::model::game::GameState;

pub fn block_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.active_player_id = Some(
        action
            .player()
            .as_ref()
            .ok_or("No player in move action")?
            .clone(),
    );
    game_state.procedure = Some(Procedure::BlockRoll);
    Ok(())
}
