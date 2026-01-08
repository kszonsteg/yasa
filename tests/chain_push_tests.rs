mod common;

use crate::common::AWAY_PLAYER_ID;
use yasa_core::actions::core::registry::ActionRegistry;
use yasa_core::model::action::Action;
use yasa_core::model::enums::{ActionType, Procedure};
use yasa_core::model::player::Player;
use yasa_core::model::position::Square;

/// Test a simple 2-player chain push: A blocks B, B pushed to C's square, C pushed to empty
#[test]
fn test_chain_push_two_players() {
    let mut state = common::game_state_setup(8, 6, 7, 7, 3, 3);
    let registry = ActionRegistry::new();

    if let Some(ref mut away_team) = state.away_team {
        let away_player_2 = Player {
            player_id: "away_player_2".to_string(),
            position: Some(Square { x: 6, y: 7 }),
            ..Default::default()
        };
        away_team
            .players_by_id
            .insert(away_player_2.player_id.clone(), away_player_2);
        let away_player_3 = Player {
            player_id: "away_player_3".to_string(),
            position: Some(Square { x: 6, y: 8 }),
            ..Default::default()
        };
        away_team
            .players_by_id
            .insert(away_player_3.player_id.clone(), away_player_3);
        let away_player_4 = Player {
            player_id: "away_player_4".to_string(),
            position: Some(Square { x: 7, y: 8 }),
            ..Default::default()
        };
        away_team
            .players_by_id
            .insert(away_player_4.player_id.clone(), away_player_4);
    }

    state.current_team_id = Some(common::HOME_TEAM_ID.to_string());
    state.active_player_id = Some(common::HOME_PLAYER_ID.to_string());
    state.procedure = Some(Procedure::BlockAction);

    registry
        .discover_actions(&mut state)
        .expect("Failed to discover block actions");

    let block_action = Action::new(ActionType::Block, None, Some(Square { x: 7, y: 7 }));
    assert!(state.available_actions.contains(&block_action));

    registry
        .execute_action(&mut state, &block_action)
        .expect("Failed to execute block");

    assert_eq!(state.procedure, Some(Procedure::BlockRoll));

    let outcomes = registry.rollout_chance_outcomes(&state).unwrap();
    let mut state = outcomes
        .into_iter()
        .find(|outcome| {
            outcome
                .resulting_state
                .rolls
                .contains(&ActionType::SelectDefenderDown)
        })
        .expect("Missing Defender Down outcome")
        .resulting_state;

    registry.discover_actions(&mut state).unwrap();
    let defender_down = Action::new(ActionType::SelectDefenderDown, None, None);
    registry.execute_action(&mut state, &defender_down).unwrap();

    assert_eq!(state.procedure, Some(Procedure::Push));
    assert!(state.block_context.as_ref().unwrap().knock_out);

    registry.discover_actions(&mut state).unwrap();
    let push_action = Action::new(ActionType::Push, None, Some(Square { x: 6, y: 7 }));
    assert!(
        state.available_actions.contains(&push_action),
        "Push to occupied square should be available. Available actions: {:?}",
        state.available_actions
    );
    registry.execute_action(&mut state, &push_action).unwrap();

    assert_eq!(state.procedure, Some(Procedure::Push));
    registry.discover_actions(&mut state).unwrap();

    let chain_push_action = Action::new(ActionType::Push, None, Some(Square { x: 5, y: 7 }));
    assert!(
        state.available_actions.contains(&chain_push_action),
        "Push to empty square should be available. Available actions: {:?}",
        state.available_actions
    );
    registry
        .execute_action(&mut state, &chain_push_action)
        .unwrap();
    assert_eq!(state.procedure, Some(Procedure::FollowUp));

    let follow_up_action = Action::new(ActionType::FollowUp, None, Some(Square { x: 7, y: 7 }));
    registry
        .execute_action(&mut state, &follow_up_action)
        .unwrap();
    assert_eq!(state.procedure, Some(Procedure::Turn));

    let home_player = state
        .get_player(&common::HOME_PLAYER_ID.to_string())
        .expect("Missing Home Player");
    assert_eq!(home_player.position, Some(Square { x: 7, y: 7 }));

    let away_player_1 = state
        .get_player(&AWAY_PLAYER_ID.to_string())
        .expect("Missing Away Player 1");
    assert_eq!(away_player_1.position, Some(Square { x: 6, y: 7 }));

    let away_player_2 = state
        .get_player(&"away_player_2".to_string())
        .expect("Missing Away Player 2");
    assert_eq!(away_player_2.position, Some(Square { x: 5, y: 7 }));
}
