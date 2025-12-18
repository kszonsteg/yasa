use super::position::Square;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathNode {
    pub position: Square,
    pub cost: f64,
    pub tackle_zones_entered: usize,
    pub moves_required: u8,
}

impl PathNode {
    pub fn new(
        position: Square,
        cost: f64,
        tackle_zones_entered: usize,
        moves_required: u8,
    ) -> Self {
        PathNode {
            position,
            cost,
            tackle_zones_entered,
            moves_required,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerPath {
    pub player_id: String,
    pub target: Square,
    pub nodes: Vec<PathNode>,
    pub current_index: usize,
}

impl PlayerPath {
    pub fn new(player_id: String, target: Square, nodes: Vec<PathNode>) -> Self {
        PlayerPath {
            player_id,
            target,
            nodes,
            current_index: 0,
        }
    }

    pub fn next_position(&self) -> Option<&Square> {
        if self.current_index + 1 < self.nodes.len() {
            Some(&self.nodes[self.current_index + 1].position)
        } else {
            None
        }
    }

    pub fn current_position(&self) -> Option<&Square> {
        self.nodes.get(self.current_index).map(|n| &n.position)
    }

    pub fn is_complete(&self) -> bool {
        self.current_index >= self.nodes.len().saturating_sub(1)
    }

    pub fn advance(&mut self) -> bool {
        if self.current_index + 1 < self.nodes.len() {
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    pub fn total_cost(&self) -> f64 {
        self.nodes.last().map(|n| n.cost).unwrap_or(0.0)
    }

    pub fn remaining_moves(&self) -> usize {
        self.nodes.len().saturating_sub(self.current_index + 1)
    }
}

pub mod costs {
    pub const BASE_MOVE_COST: f64 = 1.0;
    pub const TACKLE_ZONE_PENALTY: f64 = 5.0;
    pub const GFI_PENALTY: f64 = 2.0;
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
