use crate::model::enums::ActionType;
use crate::model::position::Square;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct Action {
    action_type: ActionType,
    player: Option<String>,
    position: Option<Square>,
}

impl Action {
    pub fn new(action_type: ActionType, player: Option<String>, position: Option<Square>) -> Self {
        Action {
            action_type,
            player,
            position,
        }
    }

    pub fn action_type(&self) -> ActionType {
        self.action_type
    }

    pub fn player(&self) -> &Option<String> {
        &self.player
    }

    pub fn position(&self) -> &Option<Square> {
        &self.position
    }
}
