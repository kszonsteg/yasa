use super::movement::move_discovery;
use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;
use crate::model::position::Square;
use std::collections::HashSet;

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

pub fn interception_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![];

    let target_position = if let Some(position) = &game_state.position {
        if position.len() == 2 {
            Square::new(position[0], position[1])
        } else {
            return Err("Wrong position format in interception discovery".to_string());
        }
    } else {
        return Err("Missing target position in interception discovery".to_string());
    };

    let passer_position = game_state
        .get_active_player()?
        .position
        .as_ref()
        .ok_or("Active player has no position in interception discovery")?;

    let current_team_id = game_state
        .current_team_id
        .as_ref()
        .ok_or("No current team id in interception discovery")?;

    let interceptors = find_interceptors(
        game_state,
        passer_position,
        &target_position,
        current_team_id,
    )?;

    for interceptor_id in interceptors {
        game_state.available_actions.push(Action::new(
            ActionType::SelectPlayer,
            Some(interceptor_id),
            None,
        ));
    }

    game_state
        .available_actions
        .push(Action::new(ActionType::SelectNone, None, None));

    Ok(())
}

fn find_interceptors(
    game_state: &GameState,
    passer_position: &Square,
    target_position: &Square,
    passing_team_id: &String,
) -> Result<Vec<String>, String> {
    let mut interceptors = Vec::new();

    let max_distance = passer_position.distance(target_position);

    // 1) Find line x from position_from to position_to using Bresenham's algorithm
    let line_squares = passer_position.create_pass_path(target_position);

    // 2) Find squares s where line intersects (already have this in line_squares)
    // 3) Include Manhattan neighbors of s into n
    // 4) Apply distance and bounding box filters
    let mut candidate_squares = HashSet::new();

    for square in &line_squares {
        let neighbors = square.get_adjacent_squares(false);
        for neighbor in neighbors {
            if
            // Remove squares where distance to passer_position or target_position is larger than max_distance
            neighbor.distance(passer_position) > max_distance
            || neighbor.distance(target_position) > max_distance
            // Remove squares outside the bounding box
            ||neighbor.x > passer_position.x.max(target_position.x)
            || neighbor.x < passer_position.x.min(target_position.x)
            || neighbor.y > passer_position.y.max(target_position.y)
            || neighbor.y < passer_position.y.min(target_position.y)
            {
                continue;
            }

            candidate_squares.insert(neighbor);
        }
        candidate_squares.insert(*square);
    }

    candidate_squares.remove(passer_position);
    candidate_squares.remove(target_position);

    // Determine the opposing team
    let opposing_team = if let Some(home_team) = &game_state.home_team {
        if &home_team.team_id == passing_team_id {
            &game_state.away_team
        } else {
            &game_state.home_team
        }
    } else {
        return Err("Missing home team at find_interceptors".to_string());
    };

    // 5) Find players on valid squares and check if they can intercept
    for square in candidate_squares {
        if let Ok(player) = game_state.get_player_at(&square) {
            if let Some(team) = opposing_team {
                // Check if player belongs to the opposing team
                if team.players_by_id.contains_key(&player.player_id) {
                    // Check if player can intercept:
                    // - must be standing
                    // - must not be stunned or knocked out
                    if player.state.up && !player.state.stunned && !player.state.knocked_out {
                        let player_id = player.player_id.clone();
                        if !interceptors.contains(&player_id) {
                            interceptors.push(player_id);
                        }
                    }
                }
            }
        }
    }

    Ok(interceptors)
}
