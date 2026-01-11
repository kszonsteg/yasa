use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;
use crate::model::position::Square;
use crate::pathfinding::Pathfinder;

pub fn move_discovery(game_state: &mut GameState) -> Result<(), String> {
    let player = game_state.get_active_player()?.clone();

    game_state.available_actions = vec![];

    if let Some(path_state) = &game_state.active_path {
        if path_state.is_complete() {
            game_state
                .available_actions
                .push(Action::new(ActionType::EndPlayerTurn, None, None));
        } else if let Some(next_square) = path_state.next_square() {
            game_state.available_actions.push(Action::new(
                ActionType::Move,
                None,
                Some(next_square),
            ));
        }
        return Ok(());
    }

    if !player.state.up {
        game_state
            .available_actions
            .push(Action::new(ActionType::StandUp, None, None));
    }

    // Use pathfinding to discover all reachable squares with optimal paths
    let pathfinder = Pathfinder::new(game_state, &player)?;
    let paths = pathfinder.find_all_paths();

    for path in paths {
        game_state.available_actions.push(Action::new_with_path(
            ActionType::Move,
            None,
            None,
            path,
        ));
    }

    game_state
        .available_actions
        .push(Action::new(ActionType::EndPlayerTurn, None, None));

    Ok(())
}

pub fn handoff_discovery(game_state: &mut GameState) -> Result<(), String> {
    move_discovery(game_state)?;

    if let Some(path_state) = &game_state.active_path {
        if !path_state.is_complete() {
            return Ok(());
        }
    }

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

    if let Some(path_state) = &game_state.active_path {
        if !path_state.is_complete() {
            return Ok(());
        }
    }

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

    if let Some(path_state) = &game_state.active_path {
        if !path_state.is_complete() {
            return Ok(());
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::enums::Procedure;
    use crate::model::player::{Player, PlayerState};
    use crate::model::team::Team;
    use std::collections::HashMap;

    fn create_test_game_state() -> GameState {
        let mut game_state = GameState::default();

        let mut home_team = Team {
            team_id: "home".to_string(),
            players_by_id: HashMap::new(),
            score: 0,
            bribes: 0,
            rerolls: 3,
        };

        let player = Player {
            player_id: "player1".to_string(),
            ma: 6,
            st: 3,
            ag: 3,
            av: 8,
            position: Some(Square::new(10, 8)),
            state: PlayerState::default(),
            ..Default::default()
        };

        home_team
            .players_by_id
            .insert("player1".to_string(), player);

        let away_team = Team {
            team_id: "away".to_string(),
            players_by_id: HashMap::new(),
            score: 0,
            bribes: 0,
            rerolls: 3,
        };

        game_state.home_team = Some(home_team);
        game_state.away_team = Some(away_team);
        game_state.current_team_id = Some("home".to_string());
        game_state.active_player_id = Some("player1".to_string());
        game_state.procedure = Some(Procedure::MoveAction);

        game_state
    }

    fn add_opponent_at(game_state: &mut GameState, pos: Square, id: &str) {
        if let Some(away_team) = &mut game_state.away_team {
            let opponent = Player {
                player_id: id.to_string(),
                ma: 6,
                st: 3,
                ag: 3,
                av: 8,
                position: Some(pos),
                state: PlayerState::default(),
                ..Default::default()
            };
            away_team.players_by_id.insert(id.to_string(), opponent);
        }
    }

    #[test]
    fn test_move_discovery_finds_all_reachable_squares() {
        let mut game_state = create_test_game_state();

        move_discovery(&mut game_state).unwrap();

        // Should have Move actions + EndPlayerTurn
        let move_actions: Vec<_> = game_state
            .available_actions
            .iter()
            .filter(|a| a.action_type() == ActionType::Move)
            .collect();

        // With MA=6 + 2 GFI, should reach many squares on empty field
        assert!(!move_actions.is_empty(), "Should find move actions");
        assert!(
            move_actions.len() > 20,
            "Should find many reachable squares, got {}",
            move_actions.len()
        );
    }

    #[test]
    fn test_move_discovery_actions_have_paths() {
        let mut game_state = create_test_game_state();

        move_discovery(&mut game_state).unwrap();

        for action in &game_state.available_actions {
            if action.action_type() == ActionType::Move {
                assert!(
                    action.path().is_some(),
                    "Move action should have a path: {:?}",
                    action
                );
                assert!(
                    action.position().is_some(),
                    "Move action should have a position: {:?}",
                    action
                );
            }
        }
    }

    #[test]
    fn test_move_discovery_paths_have_probability() {
        let mut game_state = create_test_game_state();

        move_discovery(&mut game_state).unwrap();

        for action in &game_state.available_actions {
            if action.action_type() == ActionType::Move {
                let path = action.path().unwrap();
                assert!(
                    path.prob > 0.0 && path.prob <= 1.0,
                    "Path probability should be between 0 and 1: {}",
                    path.prob
                );
            }
        }
    }

    #[test]
    fn test_move_discovery_includes_end_player_turn() {
        let mut game_state = create_test_game_state();

        move_discovery(&mut game_state).unwrap();

        let end_turn_actions: Vec<_> = game_state
            .available_actions
            .iter()
            .filter(|a| a.action_type() == ActionType::EndPlayerTurn)
            .collect();

        assert_eq!(
            end_turn_actions.len(),
            1,
            "Should have exactly one EndPlayerTurn action"
        );
    }

    #[test]
    fn test_move_discovery_with_opponents_reduces_options() {
        let mut game_state = create_test_game_state();

        // Add opponents to block some squares
        add_opponent_at(&mut game_state, Square::new(11, 8), "opp1");
        add_opponent_at(&mut game_state, Square::new(9, 8), "opp2");
        add_opponent_at(&mut game_state, Square::new(10, 7), "opp3");

        move_discovery(&mut game_state).unwrap();

        // Should not have actions to occupied squares
        for action in &game_state.available_actions {
            if action.action_type() == ActionType::Move {
                let pos = action.position().unwrap();
                assert!(
                    pos != Square::new(11, 8)
                        && pos != Square::new(9, 8)
                        && pos != Square::new(10, 7),
                    "Should not have action to occupied square: {:?}",
                    pos
                );
            }
        }
    }

    #[test]
    fn test_move_discovery_prone_player_has_stand_up() {
        let mut game_state = create_test_game_state();

        // Make player prone
        if let Some(home_team) = &mut game_state.home_team {
            home_team.players_by_id.get_mut("player1").unwrap().state.up = false;
        }

        move_discovery(&mut game_state).unwrap();

        let stand_up_actions: Vec<_> = game_state
            .available_actions
            .iter()
            .filter(|a| a.action_type() == ActionType::StandUp)
            .collect();

        assert_eq!(
            stand_up_actions.len(),
            1,
            "Prone player should have StandUp action"
        );
    }

    #[test]
    fn test_move_discovery_paths_sorted_by_safety() {
        let mut game_state = create_test_game_state();

        // Add opponent to create some risky paths
        add_opponent_at(&mut game_state, Square::new(10, 7), "opp1");

        move_discovery(&mut game_state).unwrap();

        let move_actions: Vec<_> = game_state
            .available_actions
            .iter()
            .filter(|a| a.action_type() == ActionType::Move)
            .collect();

        // First actions should have higher probability (safer)
        if move_actions.len() >= 2 {
            let first_prob = move_actions[0].path().unwrap().prob;
            let second_prob = move_actions[1].path().unwrap().prob;
            assert!(
                first_prob >= second_prob,
                "Actions should be sorted by probability: {} >= {}",
                first_prob,
                second_prob
            );
        }
    }

    use crate::model::game::PathFollowState;
    use crate::pathfinding::Path;

    #[test]
    fn test_move_discovery_respects_active_path_incomplete() {
        let mut game_state = create_test_game_state();

        let mut path = Path::new(Square::new(12, 8));
        path.squares = vec![Square::new(11, 8), Square::new(12, 8)];

        game_state.active_path = Some(PathFollowState::new(path));

        move_discovery(&mut game_state).unwrap();

        // Should only have 1 Move action to (11,8)
        assert_eq!(game_state.available_actions.len(), 1);
        let action = &game_state.available_actions[0];
        assert_eq!(action.action_type(), ActionType::Move);
        assert_eq!(action.position(), &Some(Square::new(11, 8)));
    }

    #[test]
    fn test_move_discovery_respects_active_path_complete() {
        let mut game_state = create_test_game_state();

        let mut path = Path::new(Square::new(11, 8));
        path.squares = vec![Square::new(11, 8)];

        let mut path_state = PathFollowState::new(path);
        path_state.advance(); // Make it complete
        game_state.active_path = Some(path_state);

        move_discovery(&mut game_state).unwrap();

        // Should only have EndPlayerTurn
        assert_eq!(game_state.available_actions.len(), 1);
        let action = &game_state.available_actions[0];
        assert_eq!(action.action_type(), ActionType::EndPlayerTurn);
    }
}
