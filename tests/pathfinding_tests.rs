use std::collections::HashMap;
use yasa_core::actions::pathfinding::{
    find_optimal_path, get_reachable_squares, get_reachable_squares_with_paths,
};
use yasa_core::model::game::GameState;
use yasa_core::model::pathfinding::{costs, PathNode, PlayerPath};
use yasa_core::model::player::{Player, PlayerState};
use yasa_core::model::position::Square;
use yasa_core::model::team::Team;

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_game_state_with_player(
    player_id: &str,
    position: Square,
    ma: u8,
    moves: u8,
) -> GameState {
    let mut home_players = HashMap::new();
    let player = Player {
        player_id: player_id.to_string(),
        position: Some(position),
        ma,
        state: PlayerState {
            moves,
            up: true,
            ..Default::default()
        },
        ..Default::default()
    };
    home_players.insert(player_id.to_string(), player);

    let home_team = Team {
        team_id: "home".to_string(),
        players_by_id: home_players,
        bribes: 0,
        rerolls: 3,
        score: 0,
    };

    GameState {
        home_team: Some(home_team),
        away_team: Some(Team {
            team_id: "away".to_string(),
            players_by_id: HashMap::new(),
            bribes: 0,
            rerolls: 3,
            score: 0,
        }),
        current_team_id: Some("home".to_string()),
        ..Default::default()
    }
}

fn add_opponent(game_state: &mut GameState, player_id: &str, position: Square) {
    if let Some(team) = &mut game_state.away_team {
        let opponent = Player {
            player_id: player_id.to_string(),
            position: Some(position),
            state: PlayerState {
                up: true,
                ..Default::default()
            },
            ..Default::default()
        };
        team.players_by_id.insert(player_id.to_string(), opponent);
    }
}

fn add_teammate(game_state: &mut GameState, player_id: &str, position: Square) {
    if let Some(team) = &mut game_state.home_team {
        let teammate = Player {
            player_id: player_id.to_string(),
            position: Some(position),
            state: PlayerState {
                up: true,
                ..Default::default()
            },
            ..Default::default()
        };
        team.players_by_id.insert(player_id.to_string(), teammate);
    }
}

// ============================================================================
// PathNode Tests
// ============================================================================

#[test]
fn test_path_node_creation() {
    let position = Square::new(10, 8);
    let node = PathNode::new(position, 5.0, 1, 3);

    assert_eq!(node.position, position);
    assert_eq!(node.cost, 5.0);
    assert_eq!(node.tackle_zones_entered, 1);
    assert_eq!(node.moves_required, 3);
}

#[test]
fn test_path_node_equality() {
    let node1 = PathNode::new(Square::new(10, 8), 5.0, 1, 3);
    let node2 = PathNode::new(Square::new(10, 8), 5.0, 1, 3);
    let node3 = PathNode::new(Square::new(11, 8), 5.0, 1, 3);

    assert_eq!(node1, node2);
    assert_ne!(node1, node3);
}

// ============================================================================
// PlayerPath Tests
// ============================================================================

#[test]
fn test_player_path_creation() {
    let target = Square::new(15, 8);
    let nodes = vec![
        PathNode::new(Square::new(10, 8), 0.0, 0, 0),
        PathNode::new(Square::new(11, 8), 1.0, 0, 1),
        PathNode::new(Square::new(12, 8), 2.0, 0, 2),
    ];
    let path = PlayerPath::new("player_1".to_string(), target, nodes);

    assert_eq!(path.player_id, "player_1");
    assert_eq!(path.target, target);
    assert_eq!(path.nodes.len(), 3);
    assert_eq!(path.current_index, 0);
}

#[test]
fn test_player_path_next_position() {
    let nodes = vec![
        PathNode::new(Square::new(10, 8), 0.0, 0, 0),
        PathNode::new(Square::new(11, 8), 1.0, 0, 1),
        PathNode::new(Square::new(12, 8), 2.0, 0, 2),
    ];
    let path = PlayerPath::new("player_1".to_string(), Square::new(12, 8), nodes);

    assert_eq!(path.next_position(), Some(&Square::new(11, 8)));
    assert_eq!(path.current_position(), Some(&Square::new(10, 8)));
}

#[test]
fn test_player_path_advance() {
    let nodes = vec![
        PathNode::new(Square::new(10, 8), 0.0, 0, 0),
        PathNode::new(Square::new(11, 8), 1.0, 0, 1),
        PathNode::new(Square::new(12, 8), 2.0, 0, 2),
    ];
    let mut path = PlayerPath::new("player_1".to_string(), Square::new(12, 8), nodes);

    assert!(!path.is_complete());
    assert!(path.advance()); // 0 -> 1
    assert!(!path.is_complete());
    assert!(path.advance()); // 1 -> 2
    assert!(path.is_complete());
    assert!(!path.advance()); // Can't advance past end
}

#[test]
fn test_player_path_total_cost() {
    let nodes = vec![
        PathNode::new(Square::new(10, 8), 0.0, 0, 0),
        PathNode::new(Square::new(11, 8), 1.0, 0, 1),
        PathNode::new(Square::new(12, 8), 7.0, 1, 2),
    ];
    let path = PlayerPath::new("player_1".to_string(), Square::new(12, 8), nodes);

    assert_eq!(path.total_cost(), 7.0);
}

#[test]
fn test_player_path_remaining_moves() {
    let nodes = vec![
        PathNode::new(Square::new(10, 8), 0.0, 0, 0),
        PathNode::new(Square::new(11, 8), 1.0, 0, 1),
        PathNode::new(Square::new(12, 8), 2.0, 0, 2),
    ];
    let mut path = PlayerPath::new("player_1".to_string(), Square::new(12, 8), nodes);

    assert_eq!(path.remaining_moves(), 2);
    path.advance();
    assert_eq!(path.remaining_moves(), 1);
    path.advance();
    assert_eq!(path.remaining_moves(), 0);
}

// ============================================================================
// Dijkstra Pathfinding Tests
// ============================================================================

#[test]
fn test_pathfinding_empty_field() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 0);
    let result = find_optimal_path(&game_state, "player_1", &Square::new(13, 8));

    assert!(result.is_ok());
    let path = result.unwrap();

    // Should be 4 nodes: (10,8) -> (11,8) -> (12,8) -> (13,8)
    assert_eq!(path.nodes.len(), 4);
    assert_eq!(path.nodes[0].position, Square::new(10, 8));
    assert_eq!(path.nodes[3].position, Square::new(13, 8));

    // Cost should be 3 (3 moves at 1.0 each)
    assert_eq!(path.total_cost(), 3.0);
}

#[test]
fn test_pathfinding_same_position() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 0);
    let result = find_optimal_path(&game_state, "player_1", &Square::new(10, 8));

    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.nodes.len(), 1);
    assert_eq!(path.total_cost(), 0.0);
}

#[test]
fn test_pathfinding_diagonal_movement() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 0);
    let result = find_optimal_path(&game_state, "player_1", &Square::new(13, 11));

    assert!(result.is_ok());
    let path = result.unwrap();

    // Diagonal movement: 3 squares in each direction = 3 diagonal moves
    // Path should go diagonally
    assert!(path.nodes.len() <= 4); // At most 3 diagonal moves + start
}

#[test]
fn test_pathfinding_respects_ma_limit() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 0);
    // Player has MA=6, so max is 8 moves (6 + 2 GFI)
    // Target is 10 squares away, which is unreachable
    let result = find_optimal_path(&game_state, "player_1", &Square::new(20, 8));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unreachable"));
}

#[test]
fn test_pathfinding_considers_gfi_cost() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 0);
    // Player has MA=6, target is 7 squares away (requires 1 GFI)
    let result = find_optimal_path(&game_state, "player_1", &Square::new(17, 8));

    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.nodes.len(), 8);

    // Cost: 6 moves at 1.0 = 6.0, then 7th move at 1.0 + 2.0 (GFI) = 3.0
    // Total = 9.0
    assert_eq!(path.total_cost(), 9.0);
}

#[test]
fn test_pathfinding_avoids_tackle_zones() {
    let mut game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 8, 0);

    // Add an opponent at (12, 8) which creates a tackle zone
    add_opponent(&mut game_state, "opp_1", Square::new(12, 8));

    // Path from (10,8) to (14,8) should go around the opponent
    let result = find_optimal_path(&game_state, "player_1", &Square::new(14, 8));

    assert!(result.is_ok());
    let path = result.unwrap();

    // The path should either go around (more moves, less cost) or through (fewer moves, higher cost)
    // Due to tackle zone penalty of 5.0, going around is likely cheaper
    // Verify the path is valid (each step is adjacent)
    for window in path.nodes.windows(2) {
        assert!(
            window[0].position.is_adjacent(&window[1].position),
            "Path positions {:?} and {:?} are not adjacent",
            window[0].position,
            window[1].position
        );
    }
}

#[test]
fn test_pathfinding_blocked_completely() {
    let mut game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);

    // Create a complete wall of opponents surrounding the player
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let pos = Square::new(10 + dx, 8 + dy);
            add_opponent(&mut game_state, &format!("opp_{}_{}", dx, dy), pos);
        }
    }

    // Try to reach a square far away - should be unreachable
    let result = find_optimal_path(&game_state, "player_1", &Square::new(15, 8));

    assert!(result.is_err());
}

#[test]
fn test_pathfinding_path_continuity() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 8, 0);
    let result = find_optimal_path(&game_state, "player_1", &Square::new(15, 10));

    assert!(result.is_ok());
    let path = result.unwrap();

    // Verify each square in path is adjacent to previous
    for window in path.nodes.windows(2) {
        assert!(
            window[0].position.is_adjacent(&window[1].position),
            "Positions {:?} and {:?} are not adjacent",
            window[0].position,
            window[1].position
        );
    }
}

#[test]
fn test_pathfinding_player_not_found() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 0);
    let result = find_optimal_path(&game_state, "nonexistent", &Square::new(15, 8));

    assert!(result.is_err());
}

#[test]
fn test_pathfinding_with_existing_moves() {
    // Player has already moved 3 squares, with MA=6, can move 5 more (3 normal + 2 GFI)
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 3);
    let result = find_optimal_path(&game_state, "player_1", &Square::new(15, 8));

    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.nodes.len(), 6); // 5 moves + start
}

#[test]
fn test_pathfinding_with_existing_moves_exceeds_limit() {
    // Player has already moved 7 squares, with MA=6, can only move 1 more (0 normal + 1 GFI left)
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 6, 7);
    let result = find_optimal_path(&game_state, "player_1", &Square::new(13, 8));

    // Can only move 1 square (8 - 7 = 1)
    assert!(result.is_err());
}

#[test]
fn test_pathfinding_multiple_tackle_zones() {
    let mut game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 10, 0);

    // Add multiple opponents creating overlapping tackle zones
    add_opponent(&mut game_state, "opp_1", Square::new(12, 7));
    add_opponent(&mut game_state, "opp_2", Square::new(12, 8));
    add_opponent(&mut game_state, "opp_3", Square::new(12, 9));

    let result = find_optimal_path(&game_state, "player_1", &Square::new(14, 8));

    assert!(result.is_ok());
    let path = result.unwrap();

    // Path should be valid and go around or through with appropriate cost
    assert!(path.total_cost() > 4.0); // More than base cost of 4 moves
}

// ============================================================================
// Reachability Tests
// ============================================================================

#[test]
fn test_reachable_empty_field_ma4() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);
    let result = get_reachable_squares(&game_state, "player_1");

    assert!(result.is_ok());
    let reachable = result.unwrap();

    // With MA=4 + 2 GFI = 6 moves, player can reach many squares
    assert!(!reachable.is_empty());

    // Check that we can reach at least 6 squares away
    let max_distance = reachable
        .iter()
        .map(|r| r.position.distance(&Square::new(10, 8)))
        .max()
        .unwrap_or(0);
    assert!(max_distance >= 6);
}

#[test]
fn test_reachable_excludes_starting_position() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);
    let result = get_reachable_squares(&game_state, "player_1");

    assert!(result.is_ok());
    let reachable = result.unwrap();

    // Starting position should not be in reachable squares
    assert!(
        !reachable.iter().any(|r| r.position == Square::new(10, 8)),
        "Starting position should not be in reachable squares"
    );
}

#[test]
fn test_reachable_excludes_occupied() {
    let mut game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);

    // Add a teammate at (11, 8)
    add_teammate(&mut game_state, "teammate_1", Square::new(11, 8));

    let result = get_reachable_squares(&game_state, "player_1");

    assert!(result.is_ok());
    let reachable = result.unwrap();

    // The occupied square should not be in the reachable list
    assert!(
        !reachable.iter().any(|r| r.position == Square::new(11, 8)),
        "Occupied square should not be reachable"
    );
}

#[test]
fn test_reachable_respects_ma() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 2, 0);
    let result = get_reachable_squares(&game_state, "player_1");

    assert!(result.is_ok());
    let reachable = result.unwrap();

    // With MA=2 + 2 GFI = 4 moves, can't reach more than 4 squares away
    for square in &reachable {
        let distance = square.position.distance(&Square::new(10, 8));
        assert!(
            distance <= 4,
            "Should not reach more than 4 squares away, got distance {}",
            distance
        );
    }
}

#[test]
fn test_reachable_player_not_found() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);
    let result = get_reachable_squares(&game_state, "nonexistent");

    assert!(result.is_err());
}

#[test]
fn test_reachable_sorted_by_endzone_distance() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);
    let result = get_reachable_squares(&game_state, "player_1");

    assert!(result.is_ok());
    let reachable = result.unwrap();

    // For home team, end zone is at x=1, so lower x should come first
    for window in reachable.windows(2) {
        assert!(
            window[0].position.x <= window[1].position.x,
            "Squares should be sorted by distance to end zone: {:?} should come before {:?}",
            window[0].position,
            window[1].position
        );
    }
}

#[test]
fn test_reachable_with_paths() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);
    let result = get_reachable_squares_with_paths(&game_state, "player_1");

    assert!(result.is_ok());
    let reachable = result.unwrap();

    // Each reachable square should have a valid path
    for (square, path) in &reachable {
        assert_eq!(&path.target, square);
        assert!(!path.nodes.is_empty());
        assert_eq!(path.nodes[0].position, Square::new(10, 8)); // Start position
        assert_eq!(path.nodes.last().unwrap().position, *square);
    }
}

#[test]
fn test_reachable_blocked_direction() {
    let mut game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);

    // Create a wall of opponents to the right that blocks direct path
    for y in 5..=11 {
        add_opponent(&mut game_state, &format!("opp_{}", y), Square::new(11, y));
    }

    let result = get_reachable_squares(&game_state, "player_1");

    assert!(result.is_ok());
    let reachable = result.unwrap();

    // Squares at x=11 should not be reachable (occupied by opponents)
    assert!(
        !reachable
            .iter()
            .any(|r| r.position.x == 11 && r.position.y >= 5 && r.position.y <= 11),
        "Occupied squares at x=11 should not be reachable"
    );

    // But the player can still go around if there's room (y < 5 or y > 11)
    // so some x=11 squares might be reachable if not blocked
}

// ============================================================================
// Cost Function Tests
// ============================================================================

#[test]
fn test_cost_constants() {
    assert_eq!(costs::BASE_MOVE_COST, 1.0);
    assert_eq!(costs::TACKLE_ZONE_PENALTY, 5.0);
    assert_eq!(costs::GFI_PENALTY, 2.0);
}

#[test]
fn test_tackle_zone_penalty_affects_path() {
    let mut game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 8, 0);

    // Add opponent creating tackle zone at (11, 8)
    add_opponent(&mut game_state, "opp_1", Square::new(11, 7));

    // Get path to (12, 8) - going through tackle zone vs around
    let result = find_optimal_path(&game_state, "player_1", &Square::new(12, 8));

    assert!(result.is_ok());
    let path = result.unwrap();

    // Path should exist
    assert!(path.nodes.len() >= 3);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_integration_path_follows_reachability() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);

    let reachable = get_reachable_squares(&game_state, "player_1").unwrap();

    // Every reachable square should have a valid path
    for square_info in &reachable {
        let path_result = find_optimal_path(&game_state, "player_1", &square_info.position);
        assert!(
            path_result.is_ok(),
            "Should be able to find path to reachable square {:?}",
            square_info.position
        );
    }
}

#[test]
fn test_integration_unreachable_not_in_reachability() {
    let game_state = create_test_game_state_with_player("player_1", Square::new(10, 8), 4, 0);

    // Target far away (unreachable)
    let far_target = Square::new(20, 8);

    let reachable = get_reachable_squares(&game_state, "player_1").unwrap();

    // Far target should not be in reachable squares
    assert!(
        !reachable.iter().any(|r| r.position == far_target),
        "Far target should not be reachable"
    );

    // And pathfinding should fail
    let path_result = find_optimal_path(&game_state, "player_1", &far_target);
    assert!(path_result.is_err());
}
