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

    /// Create a new TractValuePolicy with the default model path
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_path(Self::DEFAULT_MODEL_PATH)
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
            .with_output_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 1)))?
            .into_optimized()?
            .into_runnable()?;

        Ok(TractValuePolicy { model })
    }

    /// Run inference with raw spatial and non-spatial inputs
    /// Returns a single value in [-1, 1] where:
    /// - Positive values indicate home team is likely to score
    /// - Negative values indicate away team is likely to score
    pub fn infer(
        &self,
        spatial_data: &[f32],
        non_spatial_data: &[f32],
    ) -> Result<f32, Box<dyn std::error::Error>> {
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
        // Model outputs single value in [-1, 1]
        // +1 = home team scoring, -1 = away team scoring
        let value = output_view[[0, 0]];

        Ok(value)
    }
}

impl ValuePolicyTrait for TractValuePolicy {
    fn evaluate(&self, state: &GameState) -> Result<f32, Box<dyn std::error::Error>> {
        let spatial_data = InputBuilder::create_spatial_input(state);
        let non_spatial_data = InputBuilder::create_non_spatial_input(state);

        let value = self.infer(&spatial_data, &non_spatial_data)?;

        // Convert to perspective of current team
        // If home team is active, positive value is good
        // If away team is active, we need to negate
        Ok(InputBuilder::get_value_for_active_team(state, value))
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
