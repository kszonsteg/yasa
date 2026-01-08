use crate::model::position::Square;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct PushChainItem {
    pub attacker: String,
    pub defender: String,
    pub position: Option<Square>,
}

impl PushChainItem {
    pub fn new(attacker: String, defender: String, position: Option<Square>) -> Self {
        PushChainItem {
            attacker,
            defender,
            position,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct BlockContext {
    pub attacker: String,               // Player ID of the attacker/pusher
    pub defender: String,               // Player ID of the initial defender
    pub position: Square,               // Position of the blocked player
    pub knock_out: bool,                // True if the defender should be knocked out
    pub push_chain: Vec<PushChainItem>, // Stack of player IDs in the push chain
}

impl BlockContext {
    pub fn new(attacker: String, defender: String, position: Square) -> Self {
        BlockContext {
            attacker,
            defender,
            position,
            knock_out: false,
            push_chain: Vec::new(),
        }
    }
}
