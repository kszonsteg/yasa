use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;

pub fn turn_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![];

    if let Some(current_team_id) = &game_state.current_team_id {
        if let Some(team) = if game_state.is_home_team(current_team_id) {
            &game_state.home_team
        } else {
            &game_state.away_team
        } {
            let turn_state = game_state
                .turn_state
                .as_ref()
                .ok_or("Missing turn state in turn_discovery")?;
            for (player_id, player) in &team.players_by_id {
                if player.state.used
                    // not on pitch
                    || player.position.is_none()
                {
                    continue;
                }
                let player_position = player.position.ok_or("Player doesn't have position")?;

                if turn_state.blitz
                    && game_state.get_team_tackle_zones_at(&team.team_id, &player_position) > 0
                {
                    continue;
                }

                game_state.available_actions.push(Action::new(
                    ActionType::StartMove,
                    Some(player_id.clone()),
                    None,
                ));

                if turn_state.blitz_available {
                    game_state.available_actions.push(Action::new(
                        ActionType::StartBlitz,
                        Some(player_id.clone()),
                        None,
                    ));
                }

                if turn_state.pass_available {
                    game_state.available_actions.push(Action::new(
                        ActionType::StartPass,
                        Some(player_id.clone()),
                        None,
                    ));
                }

                if turn_state.handoff_available {
                    game_state.available_actions.push(Action::new(
                        ActionType::StartHandoff,
                        Some(player_id.clone()),
                        None,
                    ));
                }

                if turn_state.foul_available {
                    game_state.available_actions.push(Action::new(
                        ActionType::StartFoul,
                        Some(player_id.clone()),
                        None,
                    ));
                }

                if !turn_state.quick_snap
                    && !turn_state.blitz
                    && player.state.up
                    && game_state
                        .get_adjacent_opponents(current_team_id, &player_position)?
                        .iter()
                        .any(|opp| opp.state.up)
                {
                    game_state.available_actions.push(Action::new(
                        ActionType::StartBlock,
                        Some(player_id.clone()),
                        None,
                    ));
                }
            }
            game_state
                .available_actions
                .push(Action::new(ActionType::EndTurn, None, None));
        };
    }

    Ok(())
}
