use crate::model::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::model::enums::{Skill, WeatherType};
use crate::model::game::GameState;
use crate::model::team::Team;

// Re-export both implementations
pub mod candle_impl;
pub mod tract_impl;

// Board dimensions for the neural network
// ARENA_WIDTH=28 (x-axis), ARENA_HEIGHT=17 (y-axis)
pub const NN_BOARD_HEIGHT: usize = ARENA_HEIGHT as usize; // 17
pub const NN_BOARD_WIDTH: usize = ARENA_WIDTH as usize; // 28
pub const NUM_SPATIAL_LAYERS: usize = 27;
pub const NUM_NON_SPATIAL_FEATURES: usize = 15;

/// Trait for value policy implementations
pub trait ValuePolicyTrait: Send + Sync {
    /// Evaluate a game state and return the probability of scoring for the active team
    fn evaluate(&self, state: &GameState) -> Result<f32, Box<dyn std::error::Error>>;

    /// Get the name of this implementation for logging
    fn name(&self) -> &'static str;
}

pub struct GameEvaluator;

impl GameEvaluator {
    pub fn new() -> Self {
        GameEvaluator
    }

    /// Evaluate a game state from the perspective of the current team.
    /// Returns a score in the range [-1.0, 1.0] where:
    /// - 1.0 = definitely winning
    /// - 0.0 = draw
    /// - -1.0 = definitely losing
    pub fn evaluate(&self, _state: &GameState) -> Result<f64, String> {
        Ok(0.0)
    }
}

impl Default for GameEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating neural network inputs from game state
pub struct InputBuilder;

impl InputBuilder {
    /// Create spatial input tensor data from game state
    /// Returns a flat vector of f32 values in shape [1, NUM_SPATIAL_LAYERS, NN_BOARD_WIDTH, NN_BOARD_HEIGHT]
    /// Layout is (C, W, H) where W=28 (x-axis) and H=17 (y-axis)
    pub fn create_spatial_input(state: &GameState) -> Vec<f32> {
        let width = NN_BOARD_WIDTH;
        let height = NN_BOARD_HEIGHT;
        let num_layers = NUM_SPATIAL_LAYERS;
        // Flat array in row-major order: layer * (width * height) + x * height + y
        let mut spatial_data = vec![0.0f32; num_layers * width * height];

        // Layer 0: Ball Position
        if let Some(ball) = state.balls.first() {
            if let Some(pos) = ball.position {
                let x = pos.x as usize;
                let y = pos.y as usize;
                if x < width && y < height {
                    // Index for (C, W, H): layer * (W * H) + x * H + y
                    spatial_data[x * height + y] = 1.0;
                }
            }
        }

        // Layers 1-13: Home Team
        if let Some(home_team) = &state.home_team {
            Self::process_team_spatial(home_team, &mut spatial_data, 1, width, height);
        }

        // Layers 14-26: Away Team
        if let Some(away_team) = &state.away_team {
            Self::process_team_spatial(away_team, &mut spatial_data, 14, width, height);
        }

        spatial_data
    }

    fn process_team_spatial(
        team: &Team,
        layers: &mut [f32],
        offset: usize,
        width: usize,
        height: usize,
    ) {
        let layer_size = width * height;
        for player in team.players_by_id.values() {
            if let Some(pos) = player.position {
                let x = pos.x as usize;
                let y = pos.y as usize;
                if x >= width || y >= height {
                    continue;
                }

                // Index for (C, W, H): layer * (W * H) + x * H + y
                let base_idx = x * height + y;
                let mut set_layer = |layer: usize, val: f32| {
                    layers[(offset + layer) * layer_size + base_idx] = val;
                };

                set_layer(0, 1.0); // Player Positions
                set_layer(1, player.ma as f32);
                set_layer(2, player.st as f32);
                set_layer(3, player.ag as f32);
                set_layer(4, player.av as f32);
                set_layer(5, if player.state.up { 1.0 } else { 0.0 });
                set_layer(6, if player.state.used { 1.0 } else { 0.0 });
                set_layer(7, if player.state.stunned { 1.0 } else { 0.0 });
                set_layer(8, player.skills.contains(&Skill::Block) as u8 as f32);
                set_layer(9, player.skills.contains(&Skill::Dodge) as u8 as f32);
                set_layer(10, player.skills.contains(&Skill::SureHands) as u8 as f32);
                set_layer(11, player.skills.contains(&Skill::Catch) as u8 as f32);
                set_layer(12, player.skills.contains(&Skill::Pass) as u8 as f32);
            }
        }
    }

    /// Create non-spatial input tensor data from game state
    /// Returns a vector of f32 values in shape [1, NUM_NON_SPATIAL_FEATURES]
    pub fn create_non_spatial_input(state: &GameState) -> Vec<f32> {
        let mut features = Vec::with_capacity(NUM_NON_SPATIAL_FEATURES);

        features.push(state.half as f32);
        features.push(state.round as f32);

        features.push(state.home_team.as_ref().map_or(0.0, |t| t.rerolls as f32));
        features.push(state.home_team.as_ref().map_or(0.0, |t| t.score as f32));
        features.push(state.away_team.as_ref().map_or(0.0, |t| t.rerolls as f32));
        features.push(state.away_team.as_ref().map_or(0.0, |t| t.score as f32));

        if let Some(turn_state) = &state.turn_state {
            features.push(if turn_state.blitz_available { 1.0 } else { 0.0 });
            features.push(if turn_state.pass_available { 1.0 } else { 0.0 });
            features.push(if turn_state.handoff_available {
                1.0
            } else {
                0.0
            });
            features.push(if turn_state.foul_available { 1.0 } else { 0.0 });
        } else {
            features.extend(&[0.0, 0.0, 0.0, 0.0]);
        }

        let weather = state.weather;
        features.push((weather == WeatherType::Nice) as u8 as f32);
        features.push((weather == WeatherType::VerySunny) as u8 as f32);
        features.push((weather == WeatherType::PouringRain) as u8 as f32);
        features.push((weather == WeatherType::Blizzard) as u8 as f32);
        features.push((weather == WeatherType::SwelteringHeat) as u8 as f32);

        features
    }

    /// Determine which probability to return based on the active team
    /// Deprecated: Use get_value_for_active_team for single-output model
    pub fn get_active_team_probability(state: &GameState, home_prob: f32, away_prob: f32) -> f32 {
        match &state.current_team_id {
            Some(active_id)
                if Some(active_id.clone())
                    == state.home_team.as_ref().map(|t| t.team_id.clone()) =>
            {
                home_prob
            }
            Some(active_id)
                if Some(active_id.clone())
                    == state.away_team.as_ref().map(|t| t.team_id.clone()) =>
            {
                away_prob
            }
            _ => 0.0,
        }
    }

    /// Convert model value (from home team perspective) to active team perspective
    /// Model output: +1 = home team likely to score, -1 = away team likely to score
    /// Returns: positive = good for active team, negative = bad for active team
    pub fn get_value_for_active_team(state: &GameState, value: f32) -> f32 {
        match &state.current_team_id {
            Some(active_id)
                if Some(active_id.clone())
                    == state.home_team.as_ref().map(|t| t.team_id.clone()) =>
            {
                // Home team is active, positive value is good
                value
            }
            Some(active_id)
                if Some(active_id.clone())
                    == state.away_team.as_ref().map(|t| t.team_id.clone()) =>
            {
                // Away team is active, negate so positive = good for away
                -value
            }
            _ => 0.0,
        }
    }
}

// Re-export the implementations for convenience
pub use candle_impl::CandleValuePolicy;
pub use tract_impl::TractValuePolicy;

// Backward compatibility alias
pub type ValuePolicy = TractValuePolicy;
