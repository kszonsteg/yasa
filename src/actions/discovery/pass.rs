use super::movement::move_discovery;
use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;

pub fn pass_action_discovery(game_state: &mut GameState) -> Result<(), String> {
    move_discovery(game_state)?;
    let player_position = game_state
        .get_active_player()?
        .position
        .as_ref()
        .ok_or("Active player has no position in pass action discovery")?;

    if game_state.is_active_player_carrying_ball() {
        let (squares, _) = game_state.get_pass_distances_at(player_position)?;
        for square in squares {
            game_state
                .available_actions
                .insert(0, Action::new(ActionType::Pass, None, Some(square)));
        }
    }
    Ok(())
}
