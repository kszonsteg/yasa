use crate::model::position::Square;
use std::cmp::Ordering;

/// Internal node used by the A* pathfinding algorithm.
/// Represents a state during path exploration.
#[derive(Debug, Clone)]
pub struct PathNode {
    /// Current position in the path
    pub position: Square,
    /// Index of parent node in the closed set (None for start node)
    pub parent: Option<usize>,
    /// Actual cost to reach this node (weighted by risk)
    pub g_score: f64,
    /// Heuristic estimate to goal
    pub h_score: f64,
    /// Total score: g_score + h_score
    pub f_score: f64,
    /// Remaining movement allowance
    pub moves_left: u8,
    /// Remaining GFI attempts
    pub gfis_left: u8,
    /// Cumulative probability of reaching this node successfully
    pub prob: f64,
    /// Whether ball was picked up on the path to this node
    pub picked_up_ball: bool,
}

impl PathNode {
    pub fn new(position: Square, moves_left: u8, gfis_left: u8) -> Self {
        PathNode {
            position,
            parent: None,
            g_score: 0.0,
            h_score: 0.0,
            f_score: 0.0,
            moves_left,
            gfis_left,
            prob: 1.0,
            picked_up_ball: false,
        }
    }

    /// Create a new node from a parent with updated position and costs
    pub fn from_parent(
        parent_index: usize,
        parent: &PathNode,
        position: Square,
        move_prob: f64,
        uses_gfi: bool,
    ) -> Self {
        let moves_left = if uses_gfi {
            parent.moves_left
        } else {
            parent.moves_left.saturating_sub(1)
        };
        let gfis_left = if uses_gfi {
            parent.gfis_left.saturating_sub(1)
        } else {
            parent.gfis_left
        };

        PathNode {
            position,
            parent: Some(parent_index),
            g_score: 0.0, // Will be set by pathfinder
            h_score: 0.0, // Will be set by pathfinder
            f_score: 0.0, // Will be set by pathfinder
            moves_left,
            gfis_left,
            prob: parent.prob * move_prob,
            picked_up_ball: parent.picked_up_ball,
        }
    }

    /// Total remaining movement capacity (moves + GFIs)
    pub fn total_moves_left(&self) -> u8 {
        self.moves_left + self.gfis_left
    }

    /// Calculate heuristic (Chebyshev distance - max of dx, dy)
    pub fn calculate_heuristic(&mut self, target: &Square) {
        self.h_score = self.position.distance(target) as f64;
        self.f_score = self.g_score + self.h_score;
    }

    /// Update g_score with risk penalty
    /// Cost = steps taken + (1 - prob) * risk_weight
    pub fn update_g_score(&mut self, steps: u8, risk_weight: f64) {
        self.g_score = steps as f64 + (1.0 - self.prob) * risk_weight;
        self.f_score = self.g_score + self.h_score;
    }
}

impl PartialEq for PathNode {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Eq for PathNode {}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior (lower f_score = higher priority)
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                // Tie-breaker: prefer higher probability
                self.prob
                    .partial_cmp(&other.prob)
                    .unwrap_or(Ordering::Equal)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_node_new() {
        let node = PathNode::new(Square::new(5, 5), 6, 2);

        assert_eq!(node.position, Square::new(5, 5));
        assert_eq!(node.moves_left, 6);
        assert_eq!(node.gfis_left, 2);
        assert_eq!(node.prob, 1.0);
        assert!(node.parent.is_none());
        assert!(!node.picked_up_ball);
    }

    #[test]
    fn test_path_node_from_parent_normal_move() {
        let parent = PathNode::new(Square::new(5, 5), 6, 2);
        let child = PathNode::from_parent(0, &parent, Square::new(6, 5), 1.0, false);

        assert_eq!(child.position, Square::new(6, 5));
        assert_eq!(child.moves_left, 5); // Decremented
        assert_eq!(child.gfis_left, 2); // Unchanged
        assert_eq!(child.prob, 1.0);
        assert_eq!(child.parent, Some(0));
    }

    #[test]
    fn test_path_node_from_parent_gfi_move() {
        let mut parent = PathNode::new(Square::new(5, 5), 0, 2);
        parent.prob = 1.0;
        let gfi_prob = 5.0 / 6.0; // GFI success

        let child = PathNode::from_parent(0, &parent, Square::new(6, 5), gfi_prob, true);

        assert_eq!(child.position, Square::new(6, 5));
        assert_eq!(child.moves_left, 0); // Unchanged
        assert_eq!(child.gfis_left, 1); // Decremented
        assert!((child.prob - gfi_prob).abs() < 0.001);
    }

    #[test]
    fn test_path_node_from_parent_with_dodge() {
        let parent = PathNode::new(Square::new(5, 5), 6, 2);
        let dodge_prob = 4.0 / 6.0; // Dodge on 3+

        let child = PathNode::from_parent(0, &parent, Square::new(6, 5), dodge_prob, false);

        assert!((child.prob - dodge_prob).abs() < 0.001);
    }

    #[test]
    fn test_path_node_cumulative_probability() {
        let parent = PathNode::new(Square::new(5, 5), 6, 2);
        let first_dodge = 4.0 / 6.0;
        let child1 = PathNode::from_parent(0, &parent, Square::new(6, 5), first_dodge, false);

        let second_dodge = 3.0 / 6.0;
        let child2 = PathNode::from_parent(1, &child1, Square::new(7, 5), second_dodge, false);

        let expected_prob = first_dodge * second_dodge;
        assert!((child2.prob - expected_prob).abs() < 0.001);
    }

    #[test]
    fn test_path_node_total_moves_left() {
        let node = PathNode::new(Square::new(5, 5), 4, 2);
        assert_eq!(node.total_moves_left(), 6);
    }

    #[test]
    fn test_path_node_calculate_heuristic() {
        let mut node = PathNode::new(Square::new(5, 5), 6, 2);
        node.g_score = 2.0;
        let target = Square::new(8, 5);

        node.calculate_heuristic(&target);

        assert_eq!(node.h_score, 3.0); // Distance of 3
        assert_eq!(node.f_score, 5.0); // g + h = 2 + 3
    }

    #[test]
    fn test_path_node_update_g_score() {
        let mut node = PathNode::new(Square::new(5, 5), 6, 2);
        node.prob = 0.8; // 80% success
        node.h_score = 3.0;

        node.update_g_score(2, 10.0); // 2 steps, risk weight 10

        // g_score = 2 + (1 - 0.8) * 10 = 2 + 2 = 4
        assert!((node.g_score - 4.0).abs() < 0.001);
        assert!((node.f_score - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_path_node_ordering() {
        let mut node1 = PathNode::new(Square::new(5, 5), 6, 2);
        node1.f_score = 5.0;
        node1.prob = 0.8;

        let mut node2 = PathNode::new(Square::new(6, 5), 5, 2);
        node2.f_score = 3.0;
        node2.prob = 0.9;

        // node2 has lower f_score, so it should be "greater" (higher priority in min-heap)
        assert!(node2 > node1);
    }

    #[test]
    fn test_path_node_ordering_tie_break_by_probability() {
        let mut node1 = PathNode::new(Square::new(5, 5), 6, 2);
        node1.f_score = 5.0;
        node1.prob = 0.8;

        let mut node2 = PathNode::new(Square::new(6, 5), 5, 2);
        node2.f_score = 5.0;
        node2.prob = 0.9;

        // Same f_score, node2 has higher prob so higher priority
        assert!(node2 > node1);
    }
}
