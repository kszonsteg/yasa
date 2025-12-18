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
use crate::actions::execution::block::block_execution;
use crate::actions::execution::movement::{move_execution, stand_up_execution};
use crate::actions::execution::turn::{
    end_player_turn_execution, end_turn_execution, start_blitz_execution, start_block_execution,
    start_foul_execution, start_handoff_execution, start_move_execution, start_pass_execution,
};
use crate::actions::rollout::gfi_rollout;
use crate::actions::rollout::model::RolloutOutcome;
use crate::model::action::Action;
use crate::model::enums::{ActionType, Procedure};
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

    /// Discovers actions that can be executed on the game state.
    /// Returns an error if the procedure is not supported.
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
            // Terminal - no actions to discover
            Some(Procedure::EndTurn) => Ok(()),
            Some(Procedure::Turnover) => Ok(()),
            Some(Procedure::Touchdown) => Ok(()),
            // Errors
            Some(p) => Err(format!("Procedure not supported {p:?} in action discovery")),
            _ => Err("No procedure found in actions discovery.".to_string()),
        }
    }

    /// Executes a single action on the game state.
    /// Returns an error if the action is not supported.
    pub fn execute_action(
        &self,
        game_state: &mut GameState,
        action: &Action,
    ) -> Result<(), String> {
        match action.action_type() {
            // Turn
            ActionType::StartMove => start_move_execution(game_state, action),
            ActionType::StartBlock => start_block_execution(game_state, action),
            ActionType::StartFoul => start_foul_execution(game_state, action),
            ActionType::StartBlitz => start_blitz_execution(game_state, action),
            ActionType::StartHandoff => start_handoff_execution(game_state, action),
            ActionType::StartPass => start_pass_execution(game_state, action),
            ActionType::EndPlayerTurn => end_player_turn_execution(game_state),
            ActionType::EndTurn => end_turn_execution(game_state),
            // Movement
            ActionType::Move => move_execution(game_state, action),
            ActionType::StandUp => stand_up_execution(game_state),
            // Block
            ActionType::Block => block_execution(game_state, action),
            _ => Err(format!(
                "Implement {action:?} action execution on procedure {:?} in action registry",
                game_state.procedure
            )),
        }
    }

    /// Returns a list of possible outcomes with probabilities for a game state.
    /// For example, returns the list with possible block rolls outcomes from a block action.
    pub fn rollout_chance_outcomes(
        &self,
        game_state: &GameState,
    ) -> Result<Vec<RolloutOutcome>, String> {
        match game_state.procedure {
            Some(Procedure::GFI) => gfi_rollout(game_state),
            _ => Err(format!(
                "Implement {:?} procedure rollout.",
                game_state.procedure
            )),
        }
    }
}
