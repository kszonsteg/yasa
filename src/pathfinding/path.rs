use crate::model::position::Square;

/// Represents a calculated path for a player to move from current position to target.
/// Contains the full sequence of squares and the probability of successfully completing the path.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Full sequence of squares to traverse (does NOT include starting position)
    pub squares: Vec<Square>,
    /// Final destination square
    pub target: Square,
    /// Probability of successfully completing the path (0.0 to 1.0)
    /// Accounts for all dodge and GFI rolls required
    pub prob: f64,
    /// Number of regular movement points used
    pub moves_used: u8,
    /// Number of GFI (Go For It) attempts used
    pub gfis_used: u8,
    /// Whether this path passes through the ball position (pickup always succeeds)
    pub picks_up_ball: bool,
}

impl Path {
    pub fn new(target: Square) -> Self {
        Path {
            squares: Vec::new(),
            target,
            prob: 1.0,
            moves_used: 0,
            gfis_used: 0,
            picks_up_ball: false,
        }
    }

    /// Returns the length of the path (number of steps)
    pub fn len(&self) -> usize {
        self.squares.len()
    }

    /// Returns true if the path has no steps
    pub fn is_empty(&self) -> bool {
        self.squares.is_empty()
    }

    /// Returns the total movement cost (moves + GFIs)
    pub fn total_cost(&self) -> u8 {
        self.moves_used + self.gfis_used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_new() {
        let target = Square::new(5, 5);
        let path = Path::new(target);

        assert_eq!(path.target, target);
        assert_eq!(path.prob, 1.0);
        assert_eq!(path.moves_used, 0);
        assert_eq!(path.gfis_used, 0);
        assert!(!path.picks_up_ball);
        assert!(path.is_empty());
    }

    #[test]
    fn test_path_len() {
        let mut path = Path::new(Square::new(5, 5));
        assert_eq!(path.len(), 0);

        path.squares.push(Square::new(2, 2));
        path.squares.push(Square::new(3, 3));
        path.squares.push(Square::new(4, 4));
        path.squares.push(Square::new(5, 5));

        assert_eq!(path.len(), 4);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_path_total_cost() {
        let mut path = Path::new(Square::new(5, 5));
        path.moves_used = 4;
        path.gfis_used = 2;

        assert_eq!(path.total_cost(), 6);
    }

    #[test]
    fn test_path_with_reduced_probability() {
        let mut path = Path::new(Square::new(5, 5));
        path.prob = 0.75; // 75% success chance
        path.moves_used = 3;
        path.gfis_used = 1;

        assert_eq!(path.prob, 0.75);
        assert_eq!(path.total_cost(), 4);
    }

    #[test]
    fn test_path_clone() {
        let mut path = Path::new(Square::new(5, 5));
        path.squares.push(Square::new(2, 2));
        path.prob = 0.8;
        path.moves_used = 1;
        path.picks_up_ball = true;

        let cloned = path.clone();
        assert_eq!(cloned, path);
    }
}
