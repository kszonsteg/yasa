use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;

pub fn coin_toss_flip_action_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![
        Action::new(ActionType::Heads, None, None),
        Action::new(ActionType::Tails, None, None),
    ];
    Ok(())
}

pub fn coin_toss_kick_receive_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![
        Action::new(ActionType::Kick, None, None),
        Action::new(ActionType::Receive, None, None),
    ];
    Ok(())
}

pub fn setup_discovery(game_state: &mut GameState) -> Result<(), String> {
    if game_state.current_team_id.is_none() {
        return Err("Current team is required in setup".to_string());
    }

    if game_state.current_team_id == game_state.kicking_this_drive {
        game_state.available_actions = vec![
            Action::new(ActionType::SetupFormationZone, None, None),
            Action::new(ActionType::SetupFormationSpread, None, None),
        ];
    } else {
        game_state.available_actions = vec![
            Action::new(ActionType::SetupFormationWedge, None, None),
            Action::new(ActionType::SetupFormationLine, None, None),
        ];
    }

    Ok(())
}

pub fn place_ball_discovery(game_state: &mut GameState) -> Result<(), String> {
    let positions = game_state.get_receiving_team_side_positions();

    game_state.available_actions = positions
        .into_iter()
        .map(|position| Action::new(ActionType::PlaceBall, None, Some(position)))
        .collect();
    Ok(())
}

pub fn touchback_discovery(game_state: &mut GameState) -> Result<(), String> {
    if let Some(receiving_team_id) = &game_state.receiving_this_drive {
        let receiving_team = if game_state.is_home_team(receiving_team_id) {
            &game_state.home_team
        } else {
            &game_state.away_team
        };

        if let Some(team) = receiving_team {
            game_state.available_actions = vec![];
            for (player_id, player) in &team.players_by_id {
                if player.state.up && player.position.is_some() {
                    game_state.available_actions.push(Action::new(
                        ActionType::SelectPlayer,
                        Some(player_id.clone()),
                        None,
                    ));
                }
            }
        } else {
            return Err("Missing receiving team id in touchback discovery".to_string());
        }
    }

    Ok(())
}

pub fn high_kick_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![];

    // Get the receiving team ID
    if let Some(receiving_team_id) = &game_state.receiving_this_drive {
        let receiving_team = if game_state.is_home_team(receiving_team_id) {
            &game_state.home_team
        } else {
            &game_state.away_team
        };

        if let Some(team) = receiving_team {
            let ball_position = game_state.get_ball_position()?;
            let ball_on_team_side = game_state.is_team_side(&ball_position, receiving_team_id);
            let no_player_at_ball = game_state.get_player_at(&ball_position).is_err();
            if ball_on_team_side && no_player_at_ball {
                // Add SelectPlayer actions for each available player
                for player in team.players_by_id.values() {
                    if let Some(position) = player.position {
                        if game_state.get_team_tackle_zones_at(receiving_team_id, &position) == 0 {
                            game_state.available_actions.push(Action::new(
                                ActionType::SelectPlayer,
                                Some(player.player_id.clone()),
                                None,
                            ));
                        }
                    }
                }

                game_state
                    .available_actions
                    .push(Action::new(ActionType::SelectNone, None, None));
            }
        } else {
            return Err("Missing team in high_kick_discovery".to_string());
        }
    }
    Ok(())
}
