mod common;

use common::{game_state_setup, AWAY_PLAYER_ID, HOME_PLAYER_ID};
use yasa_core::actions::core::registry::ActionRegistry;
use yasa_core::model::action::Action;
use yasa_core::model::enums::{ActionType, Procedure};
use yasa_core::model::position::Square;

#[test]
fn test_block_rollout() {
    let mut state = game_state_setup(10, 5, 10, 6, 2, 5);
    let registry = ActionRegistry::new();
    let action = Action::new(
        ActionType::StartBlock,
        Some(HOME_PLAYER_ID.to_string()),
        None,
    );
    registry
        .execute_action(&mut state, &action)
        .expect("Block action not executed correctly.");
    assert_eq!(
        state.procedure,
        Some(Procedure::BlockAction),
        "Wrong procedure in state, after Start Block"
    );

    registry
        .discover_actions(&mut state)
        .expect("Failed to discover actions after executing StartBlock actions");
    let action = Action::new(ActionType::Block, None, Some(Square { x: 10, y: 6 }));
    assert!(state.available_actions.contains(&action));

    registry
        .execute_action(&mut state, &action)
        .expect("Failed to execute the block action");

    assert_eq!(
        state.procedure,
        Some(Procedure::BlockRoll),
        "Wrong procedure in state, after executing Block"
    );
    assert!(
        state.block_context.is_some(),
        "Block context should be initialized after Block"
    );
    let block_ctx = state.block_context.as_ref().unwrap();
    assert_eq!(
        block_ctx.attacker,
        HOME_PLAYER_ID.to_string(),
        "Wrong attacker in block context, after executing Block"
    );
    assert_eq!(
        block_ctx.defender,
        AWAY_PLAYER_ID.to_string(),
        "Wrong defender in block context, after executing Block"
    );
    assert_eq!(
        block_ctx.position,
        Square::new(10, 6),
        "Wrong position in block context, after executing Block"
    );

    let results = registry
        .rollout_chance_outcomes(&state)
        .expect("Block rollout failed.");
    assert_eq!(results.len(), 5, "Dice have 5 outcomes");
}

#[test]
fn test_block_defender_down() {
    let mut state = game_state_setup(10, 5, 10, 6, 2, 5);
    let registry = ActionRegistry::new();
    let action = Action::new(
        ActionType::StartBlock,
        Some(HOME_PLAYER_ID.to_string()),
        None,
    );
    registry
        .execute_action(&mut state, &action)
        .expect("Block action not executed correctly.");

    let action = Action::new(ActionType::Block, None, Some(Square { x: 10, y: 6 }));
    registry
        .execute_action(&mut state, &action)
        .expect("Failed to execute the block action");

    let results = registry
        .rollout_chance_outcomes(&state)
        .expect("Block rollout failed.");

    let mut state = results
        .into_iter()
        .filter(|outcome| {
            outcome
                .resulting_state
                .rolls
                .contains(&ActionType::SelectDefenderDown)
        })
        .next()
        .expect("Missing state with Defender down outcome")
        .resulting_state;
    assert_eq!(
        state.procedure,
        Some(Procedure::Block),
        "Wrong procedure in state, after executing Block"
    );
    registry
        .discover_actions(&mut state)
        .expect("Failed to discover actions");

    registry
        .execute_action(
            &mut state,
            &Action::new(ActionType::SelectDefenderDown, None, None),
        )
        .expect("Failed to execute action SelectDefenderDown");

    registry
        .discover_actions(&mut state)
        .expect("Failed to discover actions");

    assert_eq!(
        state.available_actions.len(),
        3,
        "Wrong number of actions after SelectDefenderDown"
    );

    registry
        .execute_action(
            &mut state,
            &Action::new(ActionType::Push, None, Some(Square { x: 11, y: 7 })),
        )
        .expect("Failed to execute action Push");

    registry
        .discover_actions(&mut state)
        .expect("Failed to discover actions");

    assert_eq!(
        state.available_actions.len(),
        2,
        "Wrong number of actions after Push"
    );
    assert_eq!(
        state.procedure,
        Some(Procedure::FollowUp),
        "Wrong procedure in state, after executing Push"
    );

    registry
        .execute_action(
            &mut state,
            &Action::new(ActionType::FollowUp, None, Some(Square { x: 10, y: 5 })),
        )
        .expect("Failed to execute action FollowUp");

    assert_eq!(
        state.procedure,
        Some(Procedure::Turn),
        "Wrong procedure in state, after executing FollowUp"
    );

    let home_player = state
        .get_player(&HOME_PLAYER_ID.to_string())
        .expect("Missing Home Player");

    assert!(home_player.state.up, "Player must be standing");
    assert!(home_player.state.has_blocked, "Player must have blocked");
    assert_eq!(
        home_player.position,
        Some(Square { x: 10, y: 5 }),
        "Home Player must be at the correct position"
    );

    let away_player = state
        .get_player(&AWAY_PLAYER_ID.to_string())
        .expect("Missing Home Player");

    assert!(away_player.state.knocked_out, "Player must be knocked out");
    assert_eq!(
        away_player.position,
        Some(Square { x: 11, y: 7 }),
        "Away Player must be at the correct position"
    );
}
