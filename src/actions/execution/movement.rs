use crate::actions::common::execute_player_movement;
use crate::model::action::Action;
use crate::model::enums::Procedure;
use crate::model::game::{GameState, PathFollowState};
use crate::model::position::Square;

pub fn move_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    if game_state.active_path.is_none() {
        if let Some(path) = action.path() {
            game_state.active_path = Some(PathFollowState::new(path.clone()));
        }
    }

    let position = if let Some(ref path_state) = game_state.active_path {
        path_state
            .next_square()
            .ok_or("Path is complete but move_execution called")?
    } else {
        // Fallback for actions without paths (legacy support shouldn't happen)
        action
            .position()
            .ok_or("Position missing in Move action and no active path")?
    };

    let current_team_id = game_state
        .current_team_id
        .clone()
        .ok_or("Missing current team id")?;

    // Check if GFI is required
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

    {
        let active_player = game_state.get_active_player()?;
        let player_pos = active_player
            .position
            .ok_or("Active player missing position")?;

        if game_state.get_team_tackle_zones_at(&current_team_id, &player_pos) > 0 {
            game_state.procedure = Some(Procedure::Dodge);
            game_state.position = Some(Square::new(position.x, position.y));
            return Ok(());
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::enums::ActionType;
    use crate::model::player::{Player, PlayerState};
    use crate::model::position::Square;
    use crate::model::team::Team;
    use crate::pathfinding::Path;
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
        game_state.parent_procedure = Some(Procedure::Turn);

        game_state
    }

    #[test]
    fn test_move_execution_initializes_path() {
        let mut game_state = create_test_game_state();

        let mut path = Path::new(Square::new(13, 8));
        path.squares = vec![Square::new(11, 8), Square::new(12, 8), Square::new(13, 8)];
        path.moves_used = 3;

        let action = Action::new_with_path(ActionType::Move, None, None, path);

        move_execution(&mut game_state, &action).unwrap();

        // Path should be initialized and first step executed
        assert!(game_state.active_path.is_some());

        let path_state = game_state.active_path.as_ref().unwrap();
        assert_eq!(path_state.current_step, 1); // Advanced after first move

        // Player should have moved to first square
        let player = game_state.get_active_player().unwrap();
        assert_eq!(player.position, Some(Square::new(11, 8)));
    }

    #[test]
    fn test_move_execution_continues_path() {
        let mut game_state = create_test_game_state();

        let mut path = Path::new(Square::new(12, 8));
        path.squares = vec![Square::new(11, 8), Square::new(12, 8)];
        path.moves_used = 2;

        let action = Action::new_with_path(ActionType::Move, None, None, path);

        // First step
        move_execution(&mut game_state, &action).unwrap();
        assert!(game_state.active_path.is_some());

        // Second step (same action, continues path)
        move_execution(&mut game_state, &action).unwrap();

        // Path should be complete but NOT cleared (waiting for EndPlayerTurn)
        assert!(game_state.active_path.is_some());
        assert!(game_state.active_path.as_ref().unwrap().is_complete());

        // Player should be at final position
        let player = game_state.get_active_player().unwrap();
        assert_eq!(player.position, Some(Square::new(12, 8)));
    }

    #[test]
    fn test_move_execution_gfi_triggers_procedure() {
        let mut game_state = create_test_game_state();

        // Use all MA first
        if let Some(home_team) = &mut game_state.home_team {
            home_team
                .players_by_id
                .get_mut("player1")
                .unwrap()
                .state
                .moves = 6; // MA is 6, so next move requires GFI
        }

        let mut path = Path::new(Square::new(11, 8));
        path.squares = vec![Square::new(11, 8)];
        path.gfis_used = 1;

        let action = Action::new_with_path(ActionType::Move, None, None, path);

        move_execution(&mut game_state, &action).unwrap();

        // Should switch to GFI procedure
        assert_eq!(game_state.procedure, Some(Procedure::GFI));
        assert_eq!(game_state.position, Some(Square::new(11, 8)));

        // Path should still be active (waiting for GFI result)
        assert!(game_state.active_path.is_some());
    }

    #[test]
    fn test_path_follow_state_remaining_steps() {
        let mut path = Path::new(Square::new(13, 8));
        path.squares = vec![Square::new(11, 8), Square::new(12, 8), Square::new(13, 8)];

        let mut state = PathFollowState::new(path);

        assert_eq!(state.remaining_steps(), 3);

        state.advance();
        assert_eq!(state.remaining_steps(), 2);

        state.advance();
        assert_eq!(state.remaining_steps(), 1);

        state.advance();
        assert_eq!(state.remaining_steps(), 0);
        assert!(state.is_complete());
    }
}
