use crate::model::constants::{ARENA_HEIGHT, ARENA_WIDTH};
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

    pub fn is_out_of_bounds(&self) -> bool {
        self.x <= 0 || self.x >= ARENA_WIDTH - 1 || self.y <= 0 || self.y >= ARENA_HEIGHT - 1
    }

    pub fn get_adjacent_squares(&self) -> Vec<Square> {
        let mut adjacent_squares = Vec::new();

        // Get adjacent squares (8 directions)
        let directions = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];

        for (dx, dy) in directions {
            let new_x = self.x + dx;
            let new_y = self.y + dy;

            // Check bounds
            if !(0..ARENA_WIDTH).contains(&new_x) || !(0..ARENA_HEIGHT).contains(&new_y) {
                continue;
            }

            adjacent_squares.push(Square::new(new_x, new_y));
        }

        adjacent_squares
    }

    pub fn create_pass_path(&self, to: &Square) -> Vec<Square> {
        let (mut x1, mut y1) = (self.x, self.y);
        let (mut x2, mut y2) = (to.x, to.y);

        let dx = x2 - x1;
        let dy = y2 - y1;

        let is_steep = dy.abs() > dx.abs();

        if is_steep {
            std::mem::swap(&mut x1, &mut y1);
            std::mem::swap(&mut x2, &mut y2);
        }

        let swapped = if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
            std::mem::swap(&mut y1, &mut y2);
            true
        } else {
            false
        };

        let dx = x2 - x1;
        let dy = y2 - y1;

        let mut error = dx / 2;
        let ystep = if y1 < y2 { 1 } else { -1 };

        let mut y = y1;
        let mut path = vec![];
        for x in x1..=x2 {
            if is_steep {
                path.push(Square::new(y, x));
            } else {
                path.push(Square::new(x, y));
            }
            error -= dy.abs();
            if error < 0 {
                y += ystep;
                error += dx;
            }
        }

        if swapped {
            path.reverse();
        }

        path
    }
}
