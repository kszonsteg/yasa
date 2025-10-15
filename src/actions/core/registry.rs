use crate::actions::discovery::block::{
    block_action_discovery, block_discovery, follow_up_discovery, push_discovery,
};
use crate::actions::discovery::movement::{
    blitz_discovery, foul_discovery, handoff_discovery, move_discovery,
};
use crate::actions::discovery::pass::{interception_discovery, pass_action_discovery};
use crate::actions::discovery::setup::{
    coin_toss_flip_action_discovery, coin_toss_kick_receive_discovery, high_kick_discovery,
    place_ball_discovery, setup_discovery, touchback_discovery,
};
use crate::actions::discovery::special::{ejection_discovery, reroll_discovery};
use crate::actions::discovery::turn::turn_discovery;
use crate::model::enums::Procedure;
use crate::model::game::GameState;

/// Registry that composes multiple action handlers
pub struct ActionRegistry {}

impl Default for ActionRegistry {
    fn default() -> Self {
        ActionRegistry::new()
    }
}

impl ActionRegistry {
    pub fn new() -> Self {
        ActionRegistry {}
    }

    pub fn discover_actions(&self, game_state: &mut GameState) -> Result<(), String> {
        match game_state.procedure {
            // Setup
            Some(Procedure::CoinTossFlip) => coin_toss_flip_action_discovery(game_state),
            Some(Procedure::CoinTossKickReceive) => coin_toss_kick_receive_discovery(game_state),
            Some(Procedure::Setup) => setup_discovery(game_state),
            Some(Procedure::PlaceBall) => place_ball_discovery(game_state),
            Some(Procedure::Touchback) => touchback_discovery(game_state),
            Some(Procedure::HighKick) => high_kick_discovery(game_state),
            // Turn
            Some(Procedure::Turn) => turn_discovery(game_state),
            // Special
            Some(Procedure::Reroll) => reroll_discovery(game_state),
            Some(Procedure::Ejection) => ejection_discovery(game_state),
            // Block
            Some(Procedure::BlockAction) => block_action_discovery(game_state),
            Some(Procedure::Block) => block_discovery(game_state),
            Some(Procedure::FollowUp) => follow_up_discovery(game_state),
            Some(Procedure::Push) => push_discovery(game_state),
            // Movement
            Some(Procedure::BlitzAction) => blitz_discovery(game_state),
            Some(Procedure::FoulAction) => foul_discovery(game_state),
            Some(Procedure::HandoffAction) => handoff_discovery(game_state),
            Some(Procedure::MoveAction) => move_discovery(game_state),
            // Pass
            Some(Procedure::PassAction) => pass_action_discovery(game_state),
            Some(Procedure::Interception) => interception_discovery(game_state),
            // Errors
            Some(p) => Err(format!("Procedure not supported {p:?} in action discovery")),
            _ => Err("No procedure found in actions discovery.".to_string()),
        }
    }
}
