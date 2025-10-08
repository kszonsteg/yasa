use super::position::Square;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Ball {
    pub position: Option<Square>,
    pub is_carried: bool,
}

impl Ball {
    pub fn new(position: Option<Square>, is_carried: bool) -> Self {
        Ball {
            position,
            is_carried,
        }
    }
}
