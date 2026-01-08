mod common;

use common::{game_state_setup, HOME_PLAYER_ID};
use yasa_core::actions::core::registry::ActionRegistry;
use yasa_core::mcts::evaluation::HeuristicValuePolicy;
use yasa_core::mcts::node::NodeType;
use yasa_core::mcts::tree::MCTSTree;
use yasa_core::model::action::Action;
use yasa_core::model::enums::{ActionType, Procedure};
use yasa_core::model::position::Square;

#[test]
fn test_mcts_setup() {
    let state = game_state_setup(15, 5, 10, 10, 15, 6);
    let tree = MCTSTree::new(state, 1.4).expect("Failed to create MCTS tree.");
    assert!(
        tree.nodes[tree.root_index].state.available_actions.len() > 0,
        "No actions available."
    );
}

#[test]
fn test_mcts_gfi() {
    let mut state = game_state_setup(15, 5, 10, 10, 15, 6);
    state.procedure = Some(Procedure::MoveAction);
    state.active_player_id = HOME_PLAYER_ID.to_string().into();

    let expected_moves_after_gfi = {
        let active_player = state
            .get_active_player_mut()
            .expect("Failed to get active player.");
        active_player.state.moves = active_player.ma;
        active_player.ma + 1
    };

    let mut tree = MCTSTree::new(state, 1.4).expect("Failed to create MCTS tree.");

    assert!(
        tree.nodes[tree.root_index].state.available_actions.len() > 0,
        "No actions available."
    );
    let selected_node = tree.select(tree.root_index);
    let expanded_node_idx = tree.expand(selected_node).expect("Failed to expand node.");
    assert!(tree.nodes.len() > 1, "No child nodes created.");

    let expanded_node = &tree.nodes[expanded_node_idx];
    assert_eq!(
        expanded_node.node_type,
        NodeType::Chance,
        "Action not executed correctly. Should be chance node."
    );
    assert_eq!(
        expanded_node.state.procedure,
        Some(Procedure::GFI),
        "Action not executed correctly. Should be GFI procedure."
    );

    tree.expand(expanded_node_idx)
        .expect("Failed to expand node.");
    assert_eq!(tree.nodes.len(), 4, "Node expansion failed.");
    assert_eq!(
        tree.nodes[expanded_node_idx].chance_children.len(),
        2,
        "Chance node expansion failed."
    );

    // Validate GFI outcomes
    let chance_children = &tree.nodes[expanded_node_idx].chance_children;

    let mut success_node = None;
    let mut failure_node = None;

    for &child_idx in chance_children {
        let child = &tree.nodes[child_idx];
        if child.state.procedure == Some(Procedure::Turnover) {
            failure_node = Some(child);
        } else {
            success_node = Some(child);
        }
    }

    let success = success_node.expect("Success outcome not found");
    let failure = failure_node.expect("Failure outcome not found");

    assert_eq!(
        success.chance_probability,
        5.0 / 6.0,
        "Success outcome probability incorrect"
    );
    assert_eq!(
        success.state.procedure,
        Some(Procedure::MoveAction),
        "Success outcome should restore parent procedure (MoveAction)"
    );
    let success_player = success
        .state
        .get_active_player()
        .expect("No active player in success outcome");
    assert_eq!(
        success_player.state.moves, expected_moves_after_gfi,
        "Success outcome should increment moves"
    );
    assert!(
        success_player.state.up,
        "Player should still be standing after successful GFI"
    );

    assert_eq!(
        failure.chance_probability,
        1.0 / 6.0,
        "Failure outcome probability incorrect"
    );
    assert_eq!(
        failure.state.procedure,
        Some(Procedure::Turnover),
        "Failure outcome should set procedure to Turnover"
    );
    let failure_player = failure
        .state
        .get_active_player()
        .expect("No active player in failure outcome");
    assert!(
        !failure_player.state.up,
        "Player should be knocked down after failed GFI"
    );
}

#[test]
fn test_mcts_dodge() {
    let mut state = game_state_setup(15, 5, 15, 4, 15, 6);
    state.procedure = Some(Procedure::MoveAction);
    state.parent_procedure = Some(Procedure::MoveAction);
    state.active_player_id = HOME_PLAYER_ID.to_string().into();

    let mut tree = MCTSTree::new(state, 1.4).expect("Failed to create MCTS tree.");

    assert!(
        tree.nodes[tree.root_index].state.available_actions.len() > 0,
        "No actions available."
    );
    let selected_node = tree.select(tree.root_index);
    let expanded_node_idx = tree.expand(selected_node).expect("Failed to expand node.");
    assert!(tree.nodes.len() > 1, "No child nodes created.");

    {
        let expanded_node = &tree.nodes[expanded_node_idx];
        assert_eq!(
            expanded_node.node_type,
            NodeType::Chance,
            "Action not executed correctly. Should be chance node."
        );
        assert_eq!(
            expanded_node.state.procedure,
            Some(Procedure::Dodge),
            "Should be a Dodge action"
        );
        assert_eq!(
            expanded_node.chance_children.len(),
            0,
            "Dodge node expanded before expansion."
        );
        assert_eq!(
            expanded_node.decision_children.len(),
            0,
            "Dodge node expanded before expansion."
        );
    }

    tree.expand(expanded_node_idx)
        .expect("Failed to expand the Dodge");

    let expanded_node = &tree.nodes[expanded_node_idx];

    assert_eq!(
        expanded_node.chance_children.len(),
        2,
        "Dodge node expansion failed."
    );
    assert_eq!(
        expanded_node.decision_children.len(),
        0,
        "Dodge node expansion failed."
    );

    let chance_children = &tree.nodes[expanded_node_idx].chance_children;

    let mut success_node = None;
    let mut failure_node = None;

    for &child_idx in chance_children {
        let child = &tree.nodes[child_idx];
        if child.state.procedure == Some(Procedure::Turnover) {
            failure_node = Some(child);
        } else {
            success_node = Some(child);
        }
    }

    let success = success_node.expect("Success outcome not found");
    let failure = failure_node.expect("Failure outcome not found");

    assert_eq!(
        success.chance_probability,
        3.0 / 6.0,
        "Success outcome probability incorrect"
    );
    assert_eq!(
        success.state.procedure,
        Some(Procedure::MoveAction),
        "Success outcome should restore parent procedure (MoveAction)"
    );
    let success_player = success
        .state
        .get_active_player()
        .expect("No active player in success outcome");

    assert!(
        success_player.state.up,
        "Player should still be standing after successful Dodge"
    );

    assert_eq!(
        failure.chance_probability,
        3.0 / 6.0,
        "Failure outcome probability incorrect"
    );
    assert_eq!(
        failure.state.procedure,
        Some(Procedure::Turnover),
        "Failure outcome should set procedure to Turnover"
    );
    let failure_player = failure
        .state
        .get_active_player()
        .expect("No active player in failure outcome");
    assert!(
        !failure_player.state.up,
        "Player should be knocked down after failed Dodge"
    );
}

#[test]
fn test_touchdown() {
    let mut state = game_state_setup(3, 5, 10, 10, 2, 5);
    state.procedure = Some(Procedure::MoveAction);
    state.parent_procedure = Some(Procedure::MoveAction);
    state.active_player_id = HOME_PLAYER_ID.to_string().into();

    let registry = ActionRegistry::new();
    let evaluator = HeuristicValuePolicy::new().expect("Failed to create heuristic evaluator.");

    let first_score = evaluator
        .evaluate(&state)
        .expect("Failed to evaluate state.");

    registry
        .discover_actions(&mut state)
        .expect("Failed to discover actions.");
    registry
        .execute_action(
            &mut state,
            &Action::new(ActionType::Move, None, Some(Square { x: 2, y: 5 })),
        )
        .expect("Failed to execute action.");

    assert!(
        state.balls.first().expect("Missing ball").is_carried,
        "Ball not carried."
    );
    let ball_picked_score = evaluator
        .evaluate(&state)
        .expect("Failed to evaluate state.");
    assert!(ball_picked_score > first_score, "Ball pickup not scoring.");

    registry
        .execute_action(
            &mut state,
            &Action::new(ActionType::Move, None, Some(Square { x: 1, y: 5 })),
        )
        .expect("Failed to execute action.");
    assert_eq!(
        state.home_team.as_ref().unwrap().score,
        1,
        "Home team score not updated."
    );
    assert_eq!(
        state.procedure.unwrap(),
        Procedure::Touchdown,
        "Touchdown not executed."
    );
    let touchdown_score = evaluator
        .evaluate(&state)
        .expect("Failed to evaluate state.");
    assert!(
        touchdown_score > ball_picked_score,
        "Touchdown not scoring."
    );
}
