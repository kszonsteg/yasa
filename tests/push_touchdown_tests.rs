mod common;

use crate::common::AWAY_PLAYER_ID;
use yasa_core::actions::core::registry::ActionRegistry;
use yasa_core::model::action::Action;
use yasa_core::model::enums::{ActionType, Procedure};
use yasa_core::model::position::Square;

#[test]
fn test_push_into_touchdown() {
    let mut state = common::game_state_setup(2, 7, 3, 7, 2, 7);

    state.balls[0].is_carried = true;

    state.current_team_id = Some(common::AWAY_TEAM_ID.to_string());
    state.active_player_id = Some(AWAY_PLAYER_ID.to_string());
    state.procedure = Some(Procedure::BlockAction);

    let registry = ActionRegistry::new();

    registry.discover_actions(&mut state).unwrap();
    let block_action = Action::new(ActionType::Block, None, Some(Square { x: 2, y: 7 }));
    assert!(state.available_actions.contains(&block_action));
    registry.execute_action(&mut state, &block_action).unwrap();

    let outcomes = registry.rollout_chance_outcomes(&state).unwrap();
    let mut state = outcomes
        .into_iter()
        .find(|outcome| {
            outcome
                .resulting_state
                .rolls
                .contains(&ActionType::SelectPush)
        })
        .expect("Missing Push outcome")
        .resulting_state;

    registry.discover_actions(&mut state).unwrap();
    let push_select = Action::new(ActionType::SelectPush, None, None);
    registry.execute_action(&mut state, &push_select).unwrap();

    registry.discover_actions(&mut state).unwrap();
    let push_dir = Action::new(ActionType::Push, None, Some(Square { x: 1, y: 7 }));
    assert!(
        state.available_actions.contains(&push_dir),
        "Push to 1,7 should be available"
    );

    registry.execute_action(&mut state, &push_dir).unwrap();

    assert_eq!(state.procedure, Some(Procedure::Touchdown));
    let home_score = state.home_team.as_ref().unwrap().score;
    assert_eq!(home_score, 1);
    assert_eq!(state.balls[0].position, Some(Square { x: 1, y: 7 }));
}
