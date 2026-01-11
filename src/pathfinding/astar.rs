use std::collections::{BinaryHeap, HashMap};

use crate::model::constants::{
    AGILITY_TABLE, ARENA_HEIGHT, ARENA_WIDTH, GFI_TARGET_BLIZZARD, GFI_TARGET_NORMAL, MAX_GFI,
};
use crate::model::enums::WeatherType;
use crate::model::game::GameState;
use crate::model::player::Player;
use crate::model::position::Square;

use super::node::PathNode;
use super::path::Path;

/// Risk weight for A* cost calculation.
/// Higher values prioritize safer paths over shorter paths.
const RISK_WEIGHT: f64 = 10.0;

/// Minimum probability threshold for considering a path.
/// Paths below this probability are discarded.
const MIN_PROB_THRESHOLD: f64 = 0.01;

/// A* Pathfinder for Blood Bowl movement.
/// Finds optimal paths considering movement allowance, GFI, and dodge requirements.
pub struct Pathfinder<'a> {
    game_state: &'a GameState,
    player: &'a Player,
    current_position: Square,
    ball_position: Option<Square>,
    opponent_team_id: String,
    is_blizzard: bool,
    is_quick_snap: bool,
    // Precomputed tackle zones for efficiency
    tzones: [[u8; ARENA_WIDTH as usize]; ARENA_HEIGHT as usize],
}

impl<'a> Pathfinder<'a> {
    /// Create a new pathfinder for the given player.
    pub fn new(game_state: &'a GameState, player: &'a Player) -> Result<Self, String> {
        let current_position = player
            .position
            .ok_or("Player must have a position for pathfinding")?;

        let current_team_id = game_state
            .current_team_id
            .as_ref()
            .ok_or("No current team id")?;

        let opponent_team_id = if game_state.is_home_team(current_team_id) {
            game_state
                .away_team
                .as_ref()
                .map(|t| t.team_id.clone())
                .ok_or("No away team")?
        } else {
            game_state
                .home_team
                .as_ref()
                .map(|t| t.team_id.clone())
                .ok_or("No home team")?
        };

        let ball_position =
            game_state
                .balls
                .first()
                .and_then(|b| if !b.is_carried { b.position } else { None });

        let is_blizzard = game_state.weather == WeatherType::Blizzard;
        let is_quick_snap = game_state
            .turn_state
            .as_ref()
            .is_some_and(|ts| ts.quick_snap);

        let mut pathfinder = Pathfinder {
            game_state,
            player,
            current_position,
            ball_position,
            opponent_team_id,
            is_blizzard,
            is_quick_snap,
            tzones: [[0; ARENA_WIDTH as usize]; ARENA_HEIGHT as usize],
        };

        pathfinder.precompute_tackle_zones();

        Ok(pathfinder)
    }

    /// Precompute tackle zones for all squares on the pitch.
    fn precompute_tackle_zones(&mut self) {
        let opp_team = if self.game_state.is_home_team(&self.opponent_team_id) {
            &self.game_state.home_team
        } else {
            &self.game_state.away_team
        };

        if let Some(team) = opp_team {
            for opponent in team.players_by_id.values() {
                if let Some(opp_pos) = &opponent.position {
                    if opponent.state.up && !opponent.state.stunned {
                        // Mark all adjacent squares
                        for dx in -1..=1 {
                            for dy in -1..=1 {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                let x = opp_pos.x + dx;
                                let y = opp_pos.y + dy;
                                if (0..ARENA_WIDTH).contains(&x) && (0..ARENA_HEIGHT).contains(&y) {
                                    self.tzones[y as usize][x as usize] += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get tackle zones at a position from precomputed cache.
    fn get_tackle_zones_at(&self, pos: &Square) -> u8 {
        if pos.x >= 0 && pos.x < ARENA_WIDTH && pos.y >= 0 && pos.y < ARENA_HEIGHT {
            self.tzones[pos.y as usize][pos.x as usize]
        } else {
            0
        }
    }

    /// Find all reachable paths from the player's current position.
    /// Returns paths sorted by probability (desc), then by remaining moves (desc).
    pub fn find_all_paths(&self) -> Vec<Path> {
        let ma = self.player.get_ma();
        let moves_used = self.player.state.moves;
        let mut moves_left = ma.saturating_sub(moves_used);
        let mut gfis_left = (ma + MAX_GFI).saturating_sub(moves_used).min(MAX_GFI);

        if self.is_quick_snap {
            moves_left = 1;
            gfis_left = 0;
        }

        let start = PathNode::new(self.current_position, moves_left, gfis_left);

        // Best node found for each position
        let mut best_nodes: HashMap<Square, PathNode> = HashMap::new();

        // Open set as a priority queue
        let mut open_set: BinaryHeap<PathNode> = BinaryHeap::new();
        open_set.push(start);

        // Closed set for reconstructing paths
        let mut closed_set: Vec<PathNode> = Vec::new();

        while let Some(current) = open_set.pop() {
            // Skip if we've already found a better path to this position
            if let Some(existing) = best_nodes.get(&current.position) {
                if existing.prob >= current.prob
                    && existing.total_moves_left() >= current.total_moves_left()
                {
                    continue;
                }
            }

            let current_index = closed_set.len();
            closed_set.push(current.clone());
            best_nodes.insert(current.position, current.clone());

            // Can't move further if no moves left
            if current.total_moves_left() == 0 {
                continue;
            }

            // Expand to adjacent squares
            for neighbor_pos in self.get_valid_neighbors(&current.position) {
                let (move_prob, uses_gfi) =
                    self.calculate_move_probability(&current, &neighbor_pos);

                // Skip impossible moves
                if move_prob < MIN_PROB_THRESHOLD {
                    continue;
                }

                // Check if using GFI when none left
                if uses_gfi && current.gfis_left == 0 {
                    continue;
                }

                let mut neighbor = PathNode::from_parent(
                    current_index,
                    &current,
                    neighbor_pos,
                    move_prob,
                    uses_gfi,
                );

                // Check if path picks up ball
                if Some(neighbor_pos) == self.ball_position {
                    neighbor.picked_up_ball = true;
                }

                // Calculate costs
                let steps =
                    (self.player.get_ma() - neighbor.moves_left) + (MAX_GFI - neighbor.gfis_left);
                neighbor.update_g_score(steps, RISK_WEIGHT);
                neighbor.h_score = 0.0; // Not targeting specific square
                neighbor.f_score = neighbor.g_score;

                // Only add if better than existing path to this position
                let dominated = best_nodes.get(&neighbor_pos).is_some_and(|existing| {
                    existing.prob >= neighbor.prob
                        && existing.total_moves_left() >= neighbor.total_moves_left()
                });

                if !dominated && neighbor.prob >= MIN_PROB_THRESHOLD {
                    open_set.push(neighbor);
                }
            }
        }

        // Convert closed set to paths
        self.extract_paths(&closed_set)
    }

    /// Find the best path to a specific target square.
    pub fn find_path_to(&self, target: Square) -> Option<Path> {
        let paths = self.find_all_paths();
        paths.into_iter().find(|p| p.target == target)
    }

    /// Get valid neighbor positions (on-pitch, unoccupied).
    fn get_valid_neighbors(&self, pos: &Square) -> Vec<Square> {
        let mut neighbors = Vec::new();

        for dx in -1..=1i32 {
            for dy in -1..=1i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = pos.x + dx;
                let ny = pos.y + dy;

                // Check bounds (inside pitch, not in dugout)
                if !(1..ARENA_WIDTH - 1).contains(&nx) || !(1..ARENA_HEIGHT - 1).contains(&ny) {
                    continue;
                }

                let neighbor = Square::new(nx, ny);

                // Check if occupied
                if self.game_state.get_player_at(&neighbor).is_ok() {
                    continue;
                }

                neighbors.push(neighbor);
            }
        }

        neighbors
    }

    /// Calculate the probability of successfully moving to a position.
    /// Returns (probability, uses_gfi).
    fn calculate_move_probability(&self, from_node: &PathNode, to: &Square) -> (f64, bool) {
        let uses_gfi = from_node.moves_left == 0;
        let mut prob = 1.0;

        // GFI roll if out of normal movement
        if uses_gfi {
            let gfi_target = if self.is_blizzard {
                GFI_TARGET_BLIZZARD
            } else {
                GFI_TARGET_NORMAL
            };
            prob *= (7 - gfi_target) as f64 / 6.0;
        }

        // Dodge roll if leaving tackle zones
        // Quick snap allows moving one square ignoring tackle zones
        if !self.is_quick_snap {
            let from_tzones = self.get_tackle_zones_at(&from_node.position);
            if from_tzones > 0 {
                let dodge_prob = self.calculate_dodge_probability(to);
                prob *= dodge_prob;
            }
        }

        (prob, uses_gfi)
    }

    /// Calculate dodge success probability for moving into a position.
    fn calculate_dodge_probability(&self, to: &Square) -> f64 {
        let ag = self.player.get_ag().min(6) as usize;
        let base_target = AGILITY_TABLE[ag];

        // Modifier: +1 for each tackle zone in destination
        let to_tzones = self.get_tackle_zones_at(to) as i8;
        let modifier = to_tzones;

        // Base modifier of +1 is applied
        let target = (base_target as i8 + 1 + modifier).clamp(2, 6) as u8;

        (7 - target) as f64 / 6.0
    }

    /// Extract paths from the closed set.
    fn extract_paths(&self, closed_set: &[PathNode]) -> Vec<Path> {
        let mut paths = Vec::new();

        for (idx, node) in closed_set.iter().enumerate() {
            // Skip the starting position
            if node.position == self.current_position {
                continue;
            }

            // Reconstruct the path
            let mut squares = Vec::new();
            let mut current_idx = idx;
            let mut picks_up_ball = false;

            loop {
                let current_node = &closed_set[current_idx];
                if current_node.picked_up_ball {
                    picks_up_ball = true;
                }

                if current_node.position != self.current_position {
                    squares.push(current_node.position);
                }

                if let Some(parent_idx) = current_node.parent {
                    current_idx = parent_idx;
                } else {
                    break;
                }
            }

            squares.reverse();

            let moves_used = self.player.get_ma() - node.moves_left;
            let gfis_used = MAX_GFI - node.gfis_left;

            let path = Path {
                squares,
                target: node.position,
                prob: node.prob,
                moves_used,
                gfis_used,
                picks_up_ball,
            };

            paths.push(path);
        }

        // Sort by probability (desc), then by moves remaining (desc)
        paths.sort_by(|a, b| {
            b.prob
                .partial_cmp(&a.prob)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let a_remaining = self.player.get_ma() + MAX_GFI - a.moves_used - a.gfis_used;
                    let b_remaining = self.player.get_ma() + MAX_GFI - b.moves_used - b.gfis_used;
                    b_remaining.cmp(&a_remaining)
                })
        });

        // Remove duplicate targets, keeping best path
        let mut seen_targets = std::collections::HashSet::new();
        paths.retain(|p| seen_targets.insert(p.target));

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ball::Ball;
    use crate::model::player::PlayerState;
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
            ag: 3, // AG 3 = 4+ to dodge
            av: 8,
            position: Some(Square::new(5, 5)),
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
    fn test_pathfinder_new() {
        let game_state = create_test_game_state();
        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player);
        assert!(pathfinder.is_ok());
    }

    #[test]
    fn test_pathfinder_no_position() {
        let mut game_state = create_test_game_state();
        if let Some(home_team) = &mut game_state.home_team {
            home_team.players_by_id.get_mut("player1").unwrap().position = None;
        }

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player);
        assert!(pathfinder.is_err());
    }

    #[test]
    fn test_find_all_paths_empty_field() {
        let game_state = create_test_game_state();
        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Should find paths to many squares
        assert!(!paths.is_empty());

        // All paths should have probability 1.0 (no dodges or GFIs)
        for path in &paths {
            if path.gfis_used == 0 {
                assert!(
                    (path.prob - 1.0).abs() < 0.001,
                    "Path to {:?} should have prob 1.0, got {}",
                    path.target,
                    path.prob
                );
            }
        }
    }

    #[test]
    fn test_find_all_paths_max_distance() {
        let game_state = create_test_game_state();
        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Player starts at (5,5) with MA=6, can move up to 8 squares (6 + 2 GFI)
        // Find the maximum distance reached
        let max_distance = paths
            .iter()
            .map(|p| p.target.distance(&Square::new(5, 5)))
            .max()
            .unwrap_or(0);

        assert!(
            max_distance >= 6,
            "Should reach at least 6 squares away, got {}",
            max_distance
        );
    }

    #[test]
    fn test_find_all_paths_with_gfi() {
        let game_state = create_test_game_state();
        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Find paths that use GFI
        let gfi_paths: Vec<_> = paths.iter().filter(|p| p.gfis_used > 0).collect();

        assert!(!gfi_paths.is_empty(), "Should have paths using GFI");

        // GFI paths should have probability < 1.0
        for path in gfi_paths {
            assert!(path.prob < 1.0, "GFI path should have prob < 1.0");
            // GFI is 5/6 = 0.833... per attempt
            let expected_prob = (5.0 / 6.0_f64).powi(path.gfis_used as i32);
            assert!(
                (path.prob - expected_prob).abs() < 0.001,
                "Path with {} GFIs should have prob {}, got {}",
                path.gfis_used,
                expected_prob,
                path.prob
            );
        }
    }

    #[test]
    fn test_find_all_paths_with_dodge() {
        let mut game_state = create_test_game_state();

        // Add opponent adjacent to player at (5,5)
        add_opponent_at(&mut game_state, Square::new(5, 4), "opp1");

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Paths that go through the tackle zone should have reduced probability
        let dodging_paths: Vec<_> = paths
            .iter()
            .filter(|p| p.prob < 1.0 && p.gfis_used == 0)
            .collect();

        assert!(
            !dodging_paths.is_empty(),
            "Should have paths requiring dodges"
        );
    }

    #[test]
    fn test_find_path_to_specific_target() {
        let game_state = create_test_game_state();
        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let target = Square::new(8, 5);
        let path = pathfinder.find_path_to(target);

        assert!(path.is_some(), "Should find path to {:?}", target);
        let path = path.unwrap();
        assert_eq!(path.target, target);
        assert_eq!(path.squares.len(), 3); // 3 steps from (5,5) to (8,5)
    }

    #[test]
    fn test_find_path_to_unreachable() {
        let game_state = create_test_game_state();
        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        // Target too far away (MA=6, GFI=2, max distance = 8)
        let target = Square::new(20, 5);
        let path = pathfinder.find_path_to(target);

        assert!(path.is_none(), "Should not find path to unreachable target");
    }

    #[test]
    fn test_paths_avoid_occupied_squares() {
        let mut game_state = create_test_game_state();

        // Add opponent blocking direct path
        add_opponent_at(&mut game_state, Square::new(6, 5), "opp1");

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();

        // Should not be able to move to (6,5)
        let blocked_path = pathfinder.find_path_to(Square::new(6, 5));
        assert!(blocked_path.is_none(), "Should not path through opponent");

        // Should still reach (7,5) by going around
        let around_path = pathfinder.find_path_to(Square::new(7, 5));
        assert!(around_path.is_some(), "Should find path around opponent");
    }

    #[test]
    fn test_paths_sorted_by_probability() {
        let mut game_state = create_test_game_state();

        // Add opponent to create tackle zone
        add_opponent_at(&mut game_state, Square::new(4, 4), "opp1");

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Paths should be sorted by probability descending
        for window in paths.windows(2) {
            assert!(
                window[0].prob >= window[1].prob,
                "Paths should be sorted by probability desc"
            );
        }
    }

    #[test]
    fn test_ball_pickup_path() {
        let mut game_state = create_test_game_state();

        // Add ball on the field
        game_state.balls.push(Ball {
            position: Some(Square::new(7, 5)),
            is_carried: false,
        });

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let path = pathfinder.find_path_to(Square::new(7, 5));

        assert!(path.is_some());
        let path = path.unwrap();
        assert!(
            path.picks_up_ball,
            "Path to ball should set picks_up_ball flag"
        );
    }

    #[test]
    fn test_dodge_probability_calculation() {
        let mut game_state = create_test_game_state();

        // Add opponent adjacent to create tackle zone
        add_opponent_at(&mut game_state, Square::new(5, 4), "opp1");

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();

        // AG 3 player: base dodge is 4+ (3/6 = 0.5)
        // Moving to a square with no enemy tackle zones: 4+ (50%)
        // Moving to a square with 1 enemy tackle zone: 5+ (33%)

        let path_to_clear = pathfinder.find_path_to(Square::new(6, 6));
        assert!(path_to_clear.is_some());
        let path = path_to_clear.unwrap();

        // Should have dodge probability: 4+ = 50%
        // (base 4 for AG3, +1 modifier, so need 5+ = 2/6 = 0.333 if in TZ at destination)
        // Actually moving to (6,6) from (5,5) with opponent at (5,4):
        // - Leaving (5,5) which is in TZ from (5,4), so dodge required
        // - (6,6) has 0 TZs, so dodge target = base 4 + 1 = 5, prob = 2/6
        assert!(
            path.prob < 1.0,
            "Path requiring dodge should have prob < 1.0"
        );
    }

    #[test]
    fn test_blizzard_affects_gfi() {
        let mut game_state = create_test_game_state();
        game_state.weather = WeatherType::Blizzard;

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Find a path with exactly 1 GFI
        let gfi_path = paths.iter().find(|p| p.gfis_used == 1);
        assert!(gfi_path.is_some());

        let path = gfi_path.unwrap();
        // Blizzard GFI is 4+ (4/6 = 0.666...)
        let expected_prob = 4.0 / 6.0;
        assert!(
            (path.prob - expected_prob).abs() < 0.01,
            "Blizzard GFI should be 4+, expected prob {}, got {}",
            expected_prob,
            path.prob
        );
    }

    #[test]
    fn test_player_with_moves_already_used() {
        let mut game_state = create_test_game_state();

        if let Some(home_team) = &mut game_state.home_team {
            let player = home_team.players_by_id.get_mut("player1").unwrap();
            player.state.moves = 4; // Already used 4 of 6 MA
        }

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // With 2 MA + 2 GFI remaining, max distance should be 4
        let max_distance = paths
            .iter()
            .map(|p| p.target.distance(&Square::new(5, 5)))
            .max()
            .unwrap_or(0);

        assert!(
            max_distance <= 4,
            "With 2 MA + 2 GFI, should not exceed 4 squares, got {}",
            max_distance
        );
    }

    #[test]
    fn test_unique_paths_per_target() {
        let game_state = create_test_game_state();
        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Check that each target appears only once
        let mut targets = std::collections::HashSet::new();
        for path in &paths {
            assert!(
                targets.insert(path.target),
                "Duplicate path to {:?}",
                path.target
            );
        }
    }

    #[test]
    fn test_player_with_all_ma_and_one_gfi_used() {
        let mut game_state = create_test_game_state();

        if let Some(home_team) = &mut game_state.home_team {
            let player = home_team.players_by_id.get_mut("player1").unwrap();
            player.state.moves = 7; // MA=6, used 7 (1 GFI used)
        }

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        let paths = pathfinder.find_all_paths();

        // Should have 0 MA and 1 GFI remaining
        // Max distance = 1
        let max_distance = paths
            .iter()
            .map(|p| p.target.distance(&Square::new(5, 5)))
            .max()
            .unwrap_or(0);

        assert!(
            max_distance <= 1,
            "With 7 moves used (MA=6), should only move 1 more square (last GFI), got {}",
            max_distance
        );

        // Verify correct costs in paths
        for path in paths {
            if path.len() > 0 {
                // We started with 6 MA used, so moves_used should be 6
                assert_eq!(path.moves_used, 6, "Should report 6 MA used");
                // We started with 1 GFI used, and moved 1 square (using the last GFI)
                // So total GFIs used should be 2
                assert_eq!(path.gfis_used, 2, "Should report 2 GFIs used");
            }
        }
    }

    #[test]
    fn test_player_with_excessive_moves_used() {
        let mut game_state = create_test_game_state();

        if let Some(home_team) = &mut game_state.home_team {
            let player = home_team.players_by_id.get_mut("player1").unwrap();
            // MA=6, MAX_GFI=2. Total max = 8.
            // Set moves to 10 to simulate corrupted state or excessive movement
            player.state.moves = 10;
        }

        let player = game_state
            .home_team
            .as_ref()
            .unwrap()
            .players_by_id
            .get("player1")
            .unwrap();

        let pathfinder = Pathfinder::new(&game_state, player).unwrap();
        // This should not panic
        let paths = pathfinder.find_all_paths();

        // Should return at least the start node (path of length 0) or similar
        // Since moves_left=0 and gfis_left=0, it cannot move anywhere.
        // It might return just the empty path (current position).

        // Max distance should be 0
        let max_distance = paths
            .iter()
            .map(|p| p.target.distance(&Square::new(5, 5))) // player1 is at 5,5
            .max()
            .unwrap_or(0);

        assert_eq!(
            max_distance, 0,
            "Should not be able to move with excessive moves used"
        );
    }
}
