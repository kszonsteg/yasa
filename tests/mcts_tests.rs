use yasa_core::actions::core::registry::ActionRegistry;
use yasa_core::mcts::evaluation::HeuristicValuePolicy;
use yasa_core::mcts::node::NodeType;
use yasa_core::mcts::tree::MCTSTree;
use yasa_core::model::action::Action;
use yasa_core::model::ball::Ball;
use yasa_core::model::enums::{ActionType, Procedure};
use yasa_core::model::game::GameState;
use yasa_core::model::player::Player;
use yasa_core::model::position::Square;
use yasa_core::model::team::Team;

const HOME_PLAYER_ID: &str = "home_player_id";
const HOME_TEAM_ID: &str = "home_team_id";
const AWAY_PLAYER_ID: &str = "away_player_id";
const AWAY_TEAM_ID: &str = "away_team_id";

fn game_state_setup(
    home_x: i32,
    home_y: i32,
    away_x: i32,
    away_y: i32,
    ball_x: i32,
    ball_y: i32,
) -> GameState {
    let mut home_team = Team::new(HOME_TEAM_ID.to_string());

    let home_player = Player {
        player_id: HOME_PLAYER_ID.to_string(),
        position: Some(Square {
            x: home_x,
            y: home_y,
        }),
        ma: 4,
        ..Default::default()
    };

    home_team
        .players_by_id
        .insert(home_player.player_id.clone(), home_player);

    let mut away_team = Team::new(AWAY_TEAM_ID.to_string());

    let away_player = Player {
        player_id: AWAY_PLAYER_ID.to_string(),
        position: Some(Square {
            x: away_x,
            y: away_y,
        }),
        ma: 4,
        ..Default::default()
    };

    away_team
        .players_by_id
        .insert(away_player.player_id.clone(), away_player);

    let ball = Ball::new(
        Some(Square {
            x: ball_x,
            y: ball_y,
        }),
        false,
    );

    GameState {
        home_team: Some(home_team),
        away_team: Some(away_team),
        current_team_id: Some(HOME_TEAM_ID.to_string()),
        procedure: Some(Procedure::Turn),
        balls: vec![ball],
        ..Default::default()
    }
}

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
    let active_player = state
        .get_active_player_mut()
        .expect("Failed to get active player.");
    active_player.state.moves = active_player.ma;

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
    println!("{:?}", tree.generate_mermaid_graph(0));
}

#[test]
fn test_touchdown() {
    let mut state = game_state_setup(3, 5, 10, 10, 2, 5);
    state.procedure = Some(Procedure::MoveAction);
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
    println!("First score: {first_score}");
    println!("Ball pickup score: {ball_picked_score}");
    println!("Touchdown score: {touchdown_score}");
    assert!(
        touchdown_score > ball_picked_score,
        "Touchdown not scoring."
    );
}
