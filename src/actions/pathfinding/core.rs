use crate::model::constants::ARENA_WIDTH;
use crate::model::game::GameState;
use crate::model::pathfinding::{costs, PathNode, PlayerPath};
use crate::model::position::Square;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone)]
struct DijkstraState {
    position: Square,
    cost: f64,
    moves_required: u8,
    tackle_zones_entered: usize,
}

impl PartialEq for DijkstraState {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.position == other.position
    }
}

impl Eq for DijkstraState {}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: reverse ordering so lowest cost comes first
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
struct VisitedInfo {
    cost: f64,
    moves_required: u8,
    tackle_zones_entered: usize,
    parent: Option<Square>,
}

#[derive(Debug, Clone)]
pub struct ReachableSquare {
    pub position: Square,
    pub cost: f64,
    pub moves_required: u8,
}

/// Calculate the cost of moving to a square
///
/// # Arguments
/// * `game_state` - The current game state
/// * `team_id` - The ID of the team making the move
/// * `to` - The destination square
/// * `moves_so_far` - Number of moves already made
/// * `ma` - The player's movement allowance
///
/// # Returns
/// The cost of the move, including penalties for tackle zones and GFI
fn calculate_move_cost(
    game_state: &GameState,
    team_id: &str,
    to: &Square,
    moves_so_far: u8,
    ma: u8,
) -> f64 {
    let mut cost = costs::BASE_MOVE_COST;

    let tackle_zones = game_state.get_team_tackle_zones_at(&team_id.to_string(), to);
    if tackle_zones > 0 {
        cost += costs::TACKLE_ZONE_PENALTY;
    }

    if moves_so_far >= ma {
        cost += costs::GFI_PENALTY;
    }

    cost
}

/// Run Dijkstra's algorithm from a player's position.
///
/// This is the core implementation used by both `find_optimal_path` and
/// `get_reachable_squares`. It explores all reachable squares and tracks
/// the optimal path to each.
///
/// # Arguments
/// * `game_state` - The current game state
/// * `player_id` - The ID of the player
/// * `target` - Optional target square. If Some, stops when target is reached.
///
/// # Returns
/// A HashMap of all visited squares with their path information
fn run_dijkstra(
    game_state: &GameState,
    player_id: &str,
    target: Option<&Square>,
) -> Result<(Square, HashMap<Square, VisitedInfo>, u8), String> {
    let player = game_state.get_player(&player_id.to_string())?;
    let start = player
        .position
        .ok_or("Player has no position on the field")?;

    let ma = player.get_ma();
    let current_moves = player.state.moves;
    let max_moves = ma + 2; // MA + 2 GFI allowed

    let team_id = game_state.get_player_team_id(&player_id.to_string())?;

    let mut heap = BinaryHeap::new();
    let mut visited: HashMap<Square, VisitedInfo> = HashMap::new();

    heap.push(DijkstraState {
        position: start,
        cost: 0.0,
        moves_required: current_moves,
        tackle_zones_entered: 0,
    });

    visited.insert(
        start,
        VisitedInfo {
            cost: 0.0,
            moves_required: current_moves,
            tackle_zones_entered: 0,
            parent: None,
        },
    );

    while let Some(current) = heap.pop() {
        if let Some(t) = target {
            if current.position == *t {
                return Ok((start, visited, ma));
            }
        }

        if let Some(info) = visited.get(&current.position) {
            if current.cost > info.cost {
                continue;
            }
        }

        if current.moves_required >= max_moves {
            continue;
        }

        for neighbor in current.position.get_adjacent_squares() {
            if neighbor.is_out_of_bounds() {
                continue;
            }

            if game_state.get_player_at(&neighbor).is_ok() {
                continue;
            }

            let move_cost =
                calculate_move_cost(game_state, team_id, &neighbor, current.moves_required, ma);

            let new_cost = current.cost + move_cost;
            let new_moves = current.moves_required + 1;

            if new_moves > max_moves {
                continue;
            }

            let tz_at_neighbor = game_state.get_team_tackle_zones_at(team_id, &neighbor);
            let new_tackle_zones = if tz_at_neighbor > 0 {
                current.tackle_zones_entered + 1
            } else {
                current.tackle_zones_entered
            };

            let is_better = match visited.get(&neighbor) {
                Some(info) => new_cost < info.cost,
                None => true,
            };

            if is_better {
                visited.insert(
                    neighbor,
                    VisitedInfo {
                        cost: new_cost,
                        moves_required: new_moves,
                        tackle_zones_entered: new_tackle_zones,
                        parent: Some(current.position),
                    },
                );

                heap.push(DijkstraState {
                    position: neighbor,
                    cost: new_cost,
                    moves_required: new_moves,
                    tackle_zones_entered: new_tackle_zones,
                });
            }
        }
    }

    Ok((start, visited, ma))
}

/// Find the optimal path from a player's current position to a target square
/// using Dijkstra's algorithm with tackle zone and GFI costs.
///
/// # Arguments
/// * `game_state` - The current game state
/// * `player_id` - The ID of the player to find a path for
/// * `target` - The destination square
///
/// # Returns
/// A `PlayerPath` containing the optimal path, or an error if unreachable
pub fn find_optimal_path(
    game_state: &GameState,
    player_id: &str,
    target: &Square,
) -> Result<PlayerPath, String> {
    let player = game_state.get_player(&player_id.to_string())?;
    let start = player
        .position
        .ok_or("Player has no position on the field")?;

    if start == *target {
        return Ok(PlayerPath::new(
            player_id.to_string(),
            *target,
            vec![PathNode::new(start, 0.0, 0, 0)],
        ));
    }

    let (start_pos, visited, _) = run_dijkstra(game_state, player_id, Some(target))?;

    if !visited.contains_key(target) {
        return Err(format!(
            "Target {:?} is unreachable from {:?}",
            target, start_pos
        ));
    }

    reconstruct_path(player_id, target, &visited, start_pos)
}

fn reconstruct_path(
    player_id: &str,
    target: &Square,
    visited: &HashMap<Square, VisitedInfo>,
    start: Square,
) -> Result<PlayerPath, String> {
    let mut nodes = Vec::new();
    let mut current = *target;

    // Walk backwards from target to start
    while current != start {
        let info = visited
            .get(&current)
            .ok_or_else(|| format!("Missing visited info for {:?}", current))?;

        nodes.push(PathNode::new(
            current,
            info.cost,
            info.tackle_zones_entered,
            info.moves_required,
        ));

        current = info
            .parent
            .ok_or_else(|| format!("Missing parent for {:?}", current))?;
    }

    nodes.push(PathNode::new(start, 0.0, 0, 0));

    nodes.reverse();

    Ok(PlayerPath::new(player_id.to_string(), *target, nodes))
}

/// Get all squares reachable by a player within their movement allowance.
///
/// Uses Dijkstra's algorithm to find all squares the player can move to,
/// considering tackle zones, GFI, and occupied squares.
///
/// # Arguments
/// * `game_state` - The current game state
/// * `player_id` - The ID of the player to analyze reachability for
///
/// # Returns
/// A vector of reachable squares sorted by distance to end zone (closest first)
pub fn get_reachable_squares(
    game_state: &GameState,
    player_id: &str,
) -> Result<Vec<ReachableSquare>, String> {
    let team_id = game_state.get_player_team_id(&player_id.to_string())?;
    let is_home = game_state.is_home_team(team_id);

    let (start, visited, _) = run_dijkstra(game_state, player_id, None)?;

    let mut reachable: Vec<ReachableSquare> = visited
        .into_iter()
        .filter(|(pos, _)| *pos != start)
        .map(|(pos, info)| ReachableSquare {
            position: pos,
            cost: info.cost,
            moves_required: info.moves_required,
        })
        .collect();

    // Sort by distance to end zone (closest first)
    reachable.sort_by(|a, b| {
        let dist_a = if is_home {
            a.position.x
        } else {
            ARENA_WIDTH - 1 - a.position.x
        };
        let dist_b = if is_home {
            b.position.x
        } else {
            ARENA_WIDTH - 1 - b.position.x
        };
        dist_a.cmp(&dist_b)
    });

    Ok(reachable)
}

/// Get reachable squares with their optimal paths pre-computed.
///
/// This is useful for the MCTS integration where we want to know
/// both what squares are reachable and the best path to each.
///
/// # Arguments
/// * `game_state` - The current game state
/// * `player_id` - The ID of the player
///
/// # Returns
/// A vector of (Square, PlayerPath) tuples for all reachable squares
pub fn get_reachable_squares_with_paths(
    game_state: &GameState,
    player_id: &str,
) -> Result<Vec<(Square, PlayerPath)>, String> {
    let team_id = game_state.get_player_team_id(&player_id.to_string())?;
    let is_home = game_state.is_home_team(team_id);

    let (start, visited, _) = run_dijkstra(game_state, player_id, None)?;

    // Build paths for all reachable squares
    let mut result: Vec<(Square, PlayerPath)> = visited
        .keys()
        .filter(|pos| **pos != start)
        .filter_map(|pos| {
            reconstruct_path(player_id, pos, &visited, start)
                .ok()
                .map(|path| (*pos, path))
        })
        .collect();

    // Sort by distance to end zone
    result.sort_by(|(a, _), (b, _)| {
        let dist_a = if is_home { a.x } else { ARENA_WIDTH - 1 - a.x };
        let dist_b = if is_home { b.x } else { ARENA_WIDTH - 1 - b.x };
        dist_a.cmp(&dist_b)
    });

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::player::{Player, PlayerState};
    use crate::model::team::Team;
    use std::collections::HashMap as StdHashMap;

    fn create_test_game_state() -> GameState {
        let mut home_players = StdHashMap::new();
        let player = Player {
            player_id: "player_1".to_string(),
            position: Some(Square::new(10, 8)),
            ma: 6,
            state: PlayerState {
                moves: 0,
                up: true,
                ..Default::default()
            },
            ..Default::default()
        };
        home_players.insert("player_1".to_string(), player);

        let home_team = Team {
            team_id: "home".to_string(),
            players_by_id: home_players,
            ..Default::default()
        };

        GameState {
            home_team: Some(home_team),
            away_team: Some(Team {
                team_id: "away".to_string(),
                players_by_id: StdHashMap::new(),
                ..Default::default()
            }),
            current_team_id: Some("home".to_string()),
            ..Default::default()
        }
    }

    // === Path Finding Tests ===

    #[test]
    fn test_pathfinding_empty_field() {
        let game_state = create_test_game_state();
        let result = find_optimal_path(&game_state, "player_1", &Square::new(13, 8));

        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.nodes.len(), 4);
        assert_eq!(path.nodes[0].position, Square::new(10, 8));
        assert_eq!(path.nodes[3].position, Square::new(13, 8));
        assert_eq!(path.total_cost(), 3.0);
    }

    #[test]
    fn test_pathfinding_same_position() {
        let game_state = create_test_game_state();
        let result = find_optimal_path(&game_state, "player_1", &Square::new(10, 8));

        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.nodes.len(), 1);
        assert_eq!(path.total_cost(), 0.0);
    }

    #[test]
    fn test_pathfinding_respects_ma_limit() {
        let game_state = create_test_game_state();
        // MA=6, max 8 moves. Target 10 squares away is unreachable.
        let result = find_optimal_path(&game_state, "player_1", &Square::new(20, 8));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unreachable"));
    }

    #[test]
    fn test_pathfinding_considers_gfi_cost() {
        let game_state = create_test_game_state();
        // MA=6, target 7 squares away requires 1 GFI
        let result = find_optimal_path(&game_state, "player_1", &Square::new(17, 8));

        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.nodes.len(), 8);
        // 6 moves at 1.0 + 1 move at 3.0 (1.0 base + 2.0 GFI) = 9.0
        assert_eq!(path.total_cost(), 9.0);
    }

    #[test]
    fn test_pathfinding_path_continuity() {
        let game_state = create_test_game_state();
        let result = find_optimal_path(&game_state, "player_1", &Square::new(15, 10));

        assert!(result.is_ok());
        let path = result.unwrap();

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
        let game_state = create_test_game_state();
        let result = find_optimal_path(&game_state, "nonexistent", &Square::new(15, 8));

        assert!(result.is_err());
    }

    #[test]
    fn test_pathfinding_avoids_tackle_zones() {
        let mut game_state = create_test_game_state();

        let mut away_players = StdHashMap::new();
        let opponent = Player {
            player_id: "opp_1".to_string(),
            position: Some(Square::new(12, 8)),
            state: PlayerState {
                up: true,
                ..Default::default()
            },
            ..Default::default()
        };
        away_players.insert("opp_1".to_string(), opponent);
        game_state.away_team = Some(Team {
            team_id: "away".to_string(),
            players_by_id: away_players,
            ..Default::default()
        });

        let result = find_optimal_path(&game_state, "player_1", &Square::new(14, 8));
        assert!(result.is_ok());
    }

    // === Reachability Tests ===

    #[test]
    fn test_reachable_empty_field_ma6() {
        let game_state = create_test_game_state();
        let result = get_reachable_squares(&game_state, "player_1");

        assert!(result.is_ok());
        let reachable = result.unwrap();
        assert!(!reachable.is_empty());

        // With MA=6 + 2 GFI = 8 moves, should reach at least 8 squares away
        let max_distance = reachable
            .iter()
            .map(|r| r.position.distance(&Square::new(10, 8)))
            .max()
            .unwrap_or(0);
        assert!(max_distance >= 6);
    }

    #[test]
    fn test_reachable_excludes_starting_position() {
        let game_state = create_test_game_state();
        let result = get_reachable_squares(&game_state, "player_1");

        assert!(result.is_ok());
        let reachable = result.unwrap();

        assert!(
            !reachable.iter().any(|r| r.position == Square::new(10, 8)),
            "Starting position should not be in reachable squares"
        );
    }

    #[test]
    fn test_reachable_sorted_by_endzone() {
        let game_state = create_test_game_state();
        let result = get_reachable_squares(&game_state, "player_1");

        assert!(result.is_ok());
        let reachable = result.unwrap();

        // For home team, sorted by x ascending (end zone at x=1)
        for window in reachable.windows(2) {
            assert!(
                window[0].position.x <= window[1].position.x,
                "Should be sorted by distance to end zone"
            );
        }
    }

    #[test]
    fn test_reachable_with_paths_consistency() {
        let game_state = create_test_game_state();
        let result = get_reachable_squares_with_paths(&game_state, "player_1");

        assert!(result.is_ok());
        let reachable = result.unwrap();

        for (square, path) in &reachable {
            assert_eq!(&path.target, square);
            assert!(!path.nodes.is_empty());
            assert_eq!(path.nodes[0].position, Square::new(10, 8));
            assert_eq!(path.nodes.last().unwrap().position, *square);
        }
    }

    #[test]
    fn test_reachable_player_not_found() {
        let game_state = create_test_game_state();
        let result = get_reachable_squares(&game_state, "nonexistent");

        assert!(result.is_err());
    }
}
