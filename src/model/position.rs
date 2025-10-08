use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
pub struct Square {
    pub x: i32,
    pub y: i32,
}

impl Square {
    pub fn new(x: i32, y: i32) -> Self {
        Square { x, y }
    }

    /// Default distance calculation (max of x and y differences)
    pub fn distance(&self, other: &Square) -> u32 {
        (self.x - other.x).abs().max((self.y - other.y).abs()) as u32
    }

    /// Manhattan distance calculation (sum of x and y differences)
    pub fn manhattan_distance(&self, other: &Square) -> u32 {
        (self.x - other.x).unsigned_abs() + (self.y - other.y).unsigned_abs()
    }

    /// Returns True, if the square is adjacent (distance = 1)
    pub fn is_adjacent(&self, other: &Square) -> bool {
        self.distance(other) == 1
    }
}
