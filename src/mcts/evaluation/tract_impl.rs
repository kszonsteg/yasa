//! Tract-ONNX implementation of the Value Policy
//!
//! This module provides the ONNX-based value policy using the tract library.

use crate::model::game::GameState;
use tract_onnx::prelude::*;

use super::{
    InputBuilder, ValuePolicyTrait, NN_BOARD_HEIGHT, NN_BOARD_WIDTH, NUM_NON_SPATIAL_FEATURES,
    NUM_SPATIAL_LAYERS,
};

type Model = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

/// Value Policy implementation using tract-onnx
///
/// This implementation loads an ONNX model exported from PyTorch Lightning
/// and uses the tract library for inference.
pub struct TractValuePolicy {
    model: Model,
}

impl TractValuePolicy {
    /// Default path to the ONNX model
    pub const DEFAULT_MODEL_PATH: &'static str = "exports/blood_bowl_value_net.onnx";

    pub fn new() -> Result<Self, String> {
        Self::from_path(Self::DEFAULT_MODEL_PATH).map_err(|e| e.to_string())
    }

    /// Create a new TractValuePolicy from a specific ONNX model path
    pub fn from_path(model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let model = onnx()
            .model_for_path(model_path)?
            .with_input_fact(
                0,
                InferenceFact::dt_shape(
                    f32::datum_type(),
                    tvec!(
                        1,
                        NUM_SPATIAL_LAYERS as i64,
                        NN_BOARD_WIDTH as i64,
                        NN_BOARD_HEIGHT as i64
                    ),
                ),
            )?
            .with_input_fact(
                1,
                InferenceFact::dt_shape(
                    f32::datum_type(),
                    tvec!(1, NUM_NON_SPATIAL_FEATURES as i64),
                ),
            )?
            .with_output_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 2)))?
            .into_optimized()?
            .into_runnable()?;

        Ok(TractValuePolicy { model })
    }

    /// Run inference with raw spatial and non-spatial inputs
    /// Returns (home_value, away_value) tuple where each value is in [-1, 1]
    /// - Positive values indicate team is likely to score
    /// - Negative values indicate team is unlikely to score
    pub fn infer(
        &self,
        spatial_data: &[f32],
        non_spatial_data: &[f32],
    ) -> Result<(f32, f32), Box<dyn std::error::Error>> {
        // Shape: (batch, channels, width, height) = (1, 27, 28, 17)
        let spatial_input: Tensor = tract_ndarray::Array4::from_shape_vec(
            (1, NUM_SPATIAL_LAYERS, NN_BOARD_WIDTH, NN_BOARD_HEIGHT),
            spatial_data.to_vec(),
        )?
        .into();

        let non_spatial_input: Tensor = tract_ndarray::Array2::from_shape_vec(
            (1, NUM_NON_SPATIAL_FEATURES),
            non_spatial_data.to_vec(),
        )?
        .into();

        let result_tensor = self
            .model
            .run(tvec!(spatial_input.into(), non_spatial_input.into()))?;

        let output_view = result_tensor[0].to_array_view::<f32>()?;
        // Model outputs shape (1, 2): [Home, Away] values in [-1, 1]
        let home_value = output_view[[0, 0]];
        let away_value = output_view[[0, 1]];

        Ok((home_value, away_value))
    }

    pub fn evaluate(&self, state: &GameState) -> Result<f64, String> {
        let spatial_data = InputBuilder::create_spatial_input(state);
        let non_spatial_data = InputBuilder::create_non_spatial_input(state);

        let (home_value, away_value) = self
            .infer(&spatial_data, &non_spatial_data)
            .map_err(|e| e.to_string())?;

        // Return value for the active team
        // Model outputs [Home, Away] values directly
        // Match active team and return their value
        let value = match &state.current_team_id {
            Some(active_id)
                if Some(active_id.clone())
                    == state.home_team.as_ref().map(|t| t.team_id.clone()) =>
            {
                // Home team is active
                home_value
            }
            Some(active_id)
                if Some(active_id.clone())
                    == state.away_team.as_ref().map(|t| t.team_id.clone()) =>
            {
                // Away team is active
                away_value
            }
            _ => 0.0,
        };

        Ok(value as f64)
    }
}

impl ValuePolicyTrait for TractValuePolicy {
    fn evaluate(&self, state: &GameState) -> Result<f64, String> {
        TractValuePolicy::evaluate(self, state)
    }

    fn name(&self) -> &'static str {
        "tract-onnx"
    }
}

impl Default for TractValuePolicy {
    fn default() -> Self {
        Self::new().expect("Failed to load the ONNX model")
    }
}
