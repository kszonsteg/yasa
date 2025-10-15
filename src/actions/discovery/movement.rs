use crate::model::action::Action;
use crate::model::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::model::enums::ActionType;
use crate::model::game::GameState;
use crate::model::position::Square;

pub fn move_discovery(game_state: &mut GameState) -> Result<(), String> {
    let player = game_state.get_active_player()?.clone();
    let player_position = player
        .position
        .as_ref()
        .ok_or("Active player has no position".to_string())?;

    game_state.available_actions = vec![];
    if !player.state.up {
        game_state
            .available_actions
            .push(Action::new(ActionType::StandUp, None, None));
    }

    let moves_available = (player.ma + 2).saturating_sub(player.state.moves);
    if moves_available > 0 {
        for square in player_position.get_adjacent_squares() {
            if !(1..(ARENA_WIDTH - 1)).contains(&square.x)
                || !(1..(ARENA_HEIGHT - 1)).contains(&square.y)
            {
                continue;
            }

            if game_state.get_player_at(&square).is_err() {
                game_state.available_actions.push(Action::new(
                    ActionType::Move,
                    None,
                    Some(square),
                ));
            }
        }
    }

    game_state
        .available_actions
        .push(Action::new(ActionType::EndPlayerTurn, None, None));

    Ok(())
}

pub fn handoff_discovery(game_state: &mut GameState) -> Result<(), String> {
    move_discovery(game_state)?;

    if game_state.is_active_player_carrying_ball() {
        let team_id = game_state
            .current_team_id
            .as_ref()
            .ok_or("Missing current team id in handoff discovery")?;
        let active_player_position = game_state
            .get_active_player()?
            .position
            .ok_or("Missing active player position in handoff discvoery")?;
        let team_positions: Vec<Square> = game_state
            .get_adjacent_teammates(team_id, &active_player_position)?
            .iter()
            .filter(|teammate| teammate.state.up)
            .filter_map(|teammate| teammate.position)
            .collect();
        for team_position in team_positions {
            game_state.available_actions.insert(
                0,
                Action::new(ActionType::Handoff, None, Some(team_position)),
            );
        }
    }

    Ok(())
}

pub fn blitz_discovery(game_state: &mut GameState) -> Result<(), String> {
    move_discovery(game_state)?;

    let active_player = game_state.get_active_player()?;

    if !active_player.state.has_blocked {
        let team_id = game_state
            .current_team_id
            .as_ref()
            .ok_or("Missing current team id in handoff discovery")?;
        let active_player = game_state.get_active_player()?;
        let moves_needed = if active_player.state.up { 1 } else { 4 };
        let gfi_allowed = 2;
        if active_player.state.moves + moves_needed <= active_player.get_ma() + gfi_allowed {
            let position = active_player
                .position
                .as_ref()
                .ok_or("Active player missing position in blitz discovery")?;
            let opp_positions: Vec<Square> = game_state
                .get_adjacent_opponents(team_id, position)?
                .iter()
                .filter(|opp| opp.state.up)
                .filter_map(|opp| opp.position)
                .collect();
            for opp_position in opp_positions {
                game_state
                    .available_actions
                    .insert(0, Action::new(ActionType::Block, None, Some(opp_position)));
            }
        }
    }

    Ok(())
}

pub fn foul_discovery(game_state: &mut GameState) -> Result<(), String> {
    move_discovery(game_state)?;

    let player_id = game_state
        .active_player_id
        .as_ref()
        .ok_or("Missing active player in block discovery")?;

    let player = game_state.get_player(player_id)?;
    let player_team_id = game_state.get_player_team_id(player_id)?;

    if player.state.has_blocked {
        Err("Player already blocked in foul discovery".to_string())
    } else {
        let position = player
            .position
            .as_ref()
            .ok_or("Missing player position in block discovery")?;
        let opp_positions: Vec<Square> = game_state
            .get_adjacent_opponents(player_team_id, position)?
            .iter()
            .filter(|opp| !opp.state.up)
            .filter_map(|opp| opp.position)
            .collect();
        for opp_position in opp_positions {
            game_state
                .available_actions
                .insert(0, Action::new(ActionType::Foul, None, Some(opp_position)));
        }
        Ok(())
    }
}
