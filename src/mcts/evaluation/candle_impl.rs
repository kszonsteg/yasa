//! Candle implementation of the Value Policy
//!
//! This module provides a native Rust implementation of the value network
//! using the candle ML framework. It can load weights from SafeTensors format.

use candle_core::{DType, Device, Module, Result as CandleResult, Tensor};
use candle_nn::{conv2d, linear, Conv2d, Conv2dConfig, Linear, VarBuilder};

use crate::model::game::GameState;

use super::{
    InputBuilder, ValuePolicyTrait, NN_BOARD_HEIGHT, NN_BOARD_WIDTH, NUM_NON_SPATIAL_FEATURES,
    NUM_SPATIAL_LAYERS,
};

/// Blood Bowl Value Network implemented in Candle
///
/// Architecture optimized for fast inference:
/// - 1x1 conv to reduce channels (27 -> 16)
/// - 3x3 conv for spatial patterns (16 -> 32)
/// - Global average pooling (32, W, H) -> (32,)
/// - FC head (32 + 15 -> 64 -> 1) with tanh output
pub struct ValueNetwork {
    // Spatial reduction: 1x1 conv
    spatial_reduce_conv: Conv2d,
    // Spatial pattern: 3x3 conv
    spatial_conv: Conv2d,
    // FC head
    fc1: Linear,
    fc2: Linear,
}

impl ValueNetwork {
    /// Create a new ValueNetwork with weights from VarBuilder
    pub fn new(vb: VarBuilder) -> CandleResult<Self> {
        let conv1x1_config = Conv2dConfig {
            padding: 0,
            stride: 1,
            dilation: 1,
            groups: 1,
        };

        let conv3x3_config = Conv2dConfig {
            padding: 1,
            stride: 1,
            dilation: 1,
            groups: 1,
        };

        // Spatial reduction: 27 -> 16 channels
        let spatial_reduce_conv = conv2d(
            NUM_SPATIAL_LAYERS,
            16,
            1,
            conv1x1_config,
            vb.pp("spatial_reduce.0"),
        )?;

        // Spatial conv: 16 -> 32 channels
        let spatial_conv = conv2d(16, 32, 3, conv3x3_config, vb.pp("spatial_conv.0"))?;

        // FC head: 32 + 15 = 47 -> 64 -> 1
        let fc_input_size = 32 + NUM_NON_SPATIAL_FEATURES;
        let fc1 = linear(fc_input_size, 64, vb.pp("fc_head.0"))?;
        let fc2 = linear(64, 1, vb.pp("fc_head.2"))?;

        Ok(Self {
            spatial_reduce_conv,
            spatial_conv,
            fc1,
            fc2,
        })
    }

    /// Forward pass through the network
    pub fn forward(&self, spatial: &Tensor, non_spatial: &Tensor) -> CandleResult<Tensor> {
        // Spatial reduction with ReLU
        let x = self.spatial_reduce_conv.forward(spatial)?;
        let x = x.relu()?;

        // Spatial conv with ReLU
        let x = self.spatial_conv.forward(&x)?;
        let x = x.relu()?;

        // Global average pooling: mean over spatial dimensions
        let x = x.mean(3)?; // mean over H
        let x = x.mean(2)?; // mean over W, now shape is (N, 32)

        // Concatenate with non-spatial features
        let combined = Tensor::cat(&[&x, non_spatial], 1)?;

        // FC head with tanh output
        let x = self.fc1.forward(&combined)?;
        let x = x.relu()?;
        let logits = self.fc2.forward(&x)?;
        logits.tanh()
    }
}

/// Value Policy implementation using Candle
///
/// This implementation loads model weights from SafeTensors format
/// and performs inference using the candle ML framework.
pub struct CandleValuePolicy {
    model: ValueNetwork,
    device: Device,
}

impl CandleValuePolicy {
    /// Default path to the SafeTensors model
    pub const DEFAULT_MODEL_PATH: &'static str = "exports/blood_bowl_value_net.safetensors";

    /// Create a new CandleValuePolicy with the default model path on CPU
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_path(Self::DEFAULT_MODEL_PATH, &Device::Cpu)
    }

    /// Create a new CandleValuePolicy from a specific SafeTensors model path
    pub fn from_path(
        model_path: &str,
        device: &Device,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, device)? };

        let model = ValueNetwork::new(vb)?;

        Ok(CandleValuePolicy {
            model,
            device: device.clone(),
        })
    }

    /// Create a CandleValuePolicy with random weights (for testing/benchmarking)
    pub fn with_random_weights(device: &Device) -> Result<Self, Box<dyn std::error::Error>> {
        let varmap = candle_nn::VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, device);
        let model = ValueNetwork::new(vb)?;

        Ok(CandleValuePolicy {
            model,
            device: device.clone(),
        })
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
        let spatial = Tensor::from_slice(
            spatial_data,
            (1, NUM_SPATIAL_LAYERS, NN_BOARD_WIDTH, NN_BOARD_HEIGHT),
            &self.device,
        )?;

        let non_spatial = Tensor::from_slice(
            non_spatial_data,
            (1, NUM_NON_SPATIAL_FEATURES),
            &self.device,
        )?;

        let output = self.model.forward(&spatial, &non_spatial)?;
        let output_vec: Vec<f32> = output.flatten_all()?.to_vec1()?;

        // Model outputs single value in [-1, 1]
        Ok(output_vec[0])
    }
}

impl ValuePolicyTrait for CandleValuePolicy {
    fn evaluate(&self, state: &GameState) -> Result<f32, Box<dyn std::error::Error>> {
        let spatial_data = InputBuilder::create_spatial_input(state);
        let non_spatial_data = InputBuilder::create_non_spatial_input(state);

        let value = self.infer(&spatial_data, &non_spatial_data)?;

        // Convert to perspective of current team
        Ok(InputBuilder::get_value_for_active_team(state, value))
    }

    fn name(&self) -> &'static str {
        "candle"
    }
}

impl Default for CandleValuePolicy {
    fn default() -> Self {
        Self::new().expect("Failed to load the SafeTensors model")
    }
}
