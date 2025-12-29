use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;
use crate::model::position::Square;

pub fn block_action_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![];
    let player_id = game_state
        .active_player_id
        .as_ref()
        .ok_or("Missing active player in block discovery")?;

    let player = game_state.get_player(player_id)?;
    let player_team_id = game_state.get_player_team_id(player_id)?;

    if player.state.has_blocked {
        Err("Player already blocked in block_discovery".to_string())
    } else {
        let position = player
            .position
            .as_ref()
            .ok_or("Missing player position in block discovery")?;
        let opp_positions: Vec<Square> = game_state
            .get_adjacent_opponents(player_team_id, position)?
            .iter()
            .filter(|opp| opp.state.up)
            .filter_map(|opp| opp.position)
            .collect();

        for opp_position in opp_positions {
            game_state.available_actions.push(Action::new(
                ActionType::Block,
                None,
                Some(opp_position),
            ));
        }
        game_state
            .available_actions
            .push(Action::new(ActionType::EndPlayerTurn, None, None));
        Ok(())
    }
}

pub fn block_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![];
    for roll in &game_state.rolls {
        game_state
            .available_actions
            .push(Action::new(*roll, None, None));
    }
    Ok(())
}

pub fn push_discovery(game_state: &mut GameState) -> Result<(), String> {
    let attacker_id = game_state
        .attacker
        .as_ref()
        .ok_or("Missing attacker in Push discovery".to_string())?;

    let defender_id = game_state
        .defender
        .as_ref()
        .ok_or("Missing defender in Push discovery".to_string())?;

    let attacker = game_state.get_player(attacker_id)?;
    let attacker_position = attacker
        .position
        .as_ref()
        .ok_or("No attacker position in push discovery")?;
    let defender = game_state.get_player(defender_id)?;
    let defender_position = defender
        .position
        .as_ref()
        .ok_or("No defender postion in push discovery")?;

    // Get all adjacent squares (including out of bounds)
    let adjacent_squares = defender_position.get_adjacent_squares(false);
    let mut squares_empty = Vec::new();
    let mut squares_out = Vec::new();
    let mut all_valid_squares = Vec::new();

    for square in adjacent_squares {
        let mut include = false;

        // Check direction-based distance requirements
        if attacker_position.x == defender_position.x || attacker_position.y == defender_position.y
        {
            // Straight line push (horizontal or vertical) - use default distance (max of x,y differences)
            if attacker_position.distance(&square) >= 2 {
                include = true;
            }
        } else {
            // Diagonal push - use Manhattan distance
            if attacker_position.manhattan_distance(&square) >= 3 {
                include = true;
            }
        }

        if include {
            if square.is_out_of_bounds() {
                squares_out.push(square);
            } else if game_state.get_player_at(&square).is_err() {
                squares_empty.push(square);
            }
            all_valid_squares.push(square);
        }
    }

    let final_squares = if !squares_empty.is_empty() {
        squares_empty
    } else if !squares_out.is_empty() {
        squares_out
    } else {
        // If no empty or out-of-bounds squares, use all valid squares
        // This should not happen according to Blood Bowl rules, but included for safety
        all_valid_squares
    };

    for square in final_squares {
        game_state
            .available_actions
            .push(Action::new(ActionType::Push, None, Some(square)));
    }

    Ok(())
}

pub fn follow_up_discovery(game_state: &mut GameState) -> Result<(), String> {
    let player = game_state.get_active_player()?;
    let position = game_state
        .position
        .as_ref()
        .ok_or("No position for follow up".to_string())?;

    game_state.available_actions = vec![
        Action::new(
            ActionType::FollowUp,
            None,
            Some(Square::new(position[0], position[1])),
        ),
        Action::new(ActionType::FollowUp, None, player.position),
    ];

    Ok(())
}
