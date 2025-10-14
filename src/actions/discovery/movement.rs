use crate::model::action::Action;
use crate::model::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::model::enums::ActionType;
use crate::model::game::GameState;

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

    if game_state
        .turn_state
        .as_ref()
        .ok_or("Missing turn state in handoff discovery")?
        .handoff_available
        && game_state.is_active_player_carrying_ball()
    {
        let team_id = game_state
            .current_team_id
            .as_ref()
            .ok_or("Missing current team id in handoff discovery")?;
        let active_player_position = game_state
            .get_active_player()?
            .position
            .ok_or("Missing active player position in handoff discvoery")?;
        // TODO: Get rid of this clone
        let gs = game_state.clone();
        for teammate in gs.get_adjacent_teammates(team_id, &active_player_position)? {
            if teammate.state.up {
                game_state
                    .available_actions
                    .insert(0, Action::new(ActionType::Handoff, None, teammate.position));
            }
        }
    }

    Ok(())
}

pub fn blitz_discovery(game_state: &mut GameState) -> Result<(), String> {
    move_discovery(game_state)?;

    let active_player = game_state.get_active_player()?;

    if game_state
        .turn_state
        .as_ref()
        .ok_or("Missing turn state in handoff discovery")?
        .blitz_available
        && !active_player.state.has_blocked
    {
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
            // TODO: get rid of this copy
            let gs = game_state.clone();
            let adjacent_opponents = gs.get_adjacent_opponents(team_id, position)?;
            for opponent in adjacent_opponents {
                if opponent.state.up {
                    game_state
                        .available_actions
                        .insert(0, Action::new(ActionType::Block, None, opponent.position));
                }
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
        // TODO: Get rid of this clone
        let gs = game_state.clone();
        let opponents = gs.get_adjacent_opponents(player_team_id, position)?;

        for opp in opponents {
            if !opp.state.up {
                game_state.available_actions.push(Action::new(
                    ActionType::Foul,
                    None,
                    opp.position,
                ));
            }
        }
        Ok(())
    }
}
