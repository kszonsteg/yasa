use super::player::Player;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Team {
    pub bribes: u8,
    pub players_by_id: HashMap<String, Player>,
    pub rerolls: u8,
    pub score: u8,
    pub team_id: String,
}

impl Team {
    pub fn new(team_id: String) -> Self {
        Team {
            team_id,
            bribes: 1,
            rerolls: 3,
            players_by_id: HashMap::new(),
            score: 0,
        }
    }
}
