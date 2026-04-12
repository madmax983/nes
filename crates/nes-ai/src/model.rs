//! The Hybrid Policy-Value Network Architecture.
//!
//! This module contains the core neural network definition used by our reinforcement
//! learning agent. It is a "hybrid" network because it accepts two distinct modalities
//! of input simultaneously:
//!
//! 1. **Visual Data**: A stack of grayscale emulator frames processed by Convolutional
//!    Neural Network (CNN) layers.
//! 2. **Numeric Features**: Extracted RAM values (like player speed, coordinates)
//!    processed by dense linear layers.
//!
//! These two modalities are concatenated together into a single "trunk" representation.
//!
//! It is a "Policy-Value" network because it produces two outputs:
//!
//! 1. **Policy (Actor)**: A probability distribution across all possible controller actions.
//! 2. **Value (Critic)**: An estimate of the expected future reward from the current state.

use burn_core as burn;
use burn_core::prelude::{Backend, Config, Module, Tensor};
use burn_nn::{
    Linear, LinearConfig, Relu,
    conv::{Conv2d, Conv2dConfig},
    pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig},
};

use crate::{config::MIN_OBSERVATION_DIM, env::ObservationSnapshot};

/// Configuration parameters for initializing a `HybridPolicyValueNet`.
#[derive(Config, Debug)]
pub struct HybridPolicyValueConfig {
    /// The number of sequential grayscale frames bundled into a single observation.
    pub frame_stack: usize,
    /// The length of the numeric feature vector (e.g., extracted RAM values).
    pub feature_count: usize,
    /// The total number of discrete controller actions the network can output.
    pub action_count: usize,
    /// The horizontal resolution of the downsampled grayscale input frame.
    #[config(default = "84")]
    pub observation_width: usize,
    /// The vertical resolution of the downsampled grayscale input frame.
    #[config(default = "84")]
    pub observation_height: usize,
}

impl HybridPolicyValueConfig {
    /// Creates a new configuration builder using the dimensions from a captured observation.
    #[must_use]
    pub fn from_observation(observation: &ObservationSnapshot, action_count: usize) -> Self {
        Self::new(
            observation.frame_stack,
            observation.features.len(),
            action_count,
        )
        .with_observation_width(observation.width)
        .with_observation_height(observation.height)
    }

    /// Initializes the actual neural network module on the specified backend device.
    #[must_use]
    pub fn init<B: Backend>(&self, device: &B::Device) -> HybridPolicyValueNet<B> {
        self.validate();
        HybridPolicyValueNet::new(device, self)
    }

    fn validate(&self) {
        assert!(
            self.frame_stack > 0,
            "frame_stack must be greater than zero"
        );
        assert!(
            self.feature_count > 0,
            "feature_count must be greater than zero"
        );
        assert!(
            self.action_count > 0,
            "action_count must be greater than zero"
        );
        assert!(
            self.observation_width >= MIN_OBSERVATION_DIM,
            "observation width must be at least {MIN_OBSERVATION_DIM} pixels"
        );
        assert!(
            self.observation_height >= MIN_OBSERVATION_DIM,
            "observation height must be at least {MIN_OBSERVATION_DIM} pixels"
        );
    }
}

/// A bundled batch of input data for the hybrid network.
#[derive(Debug)]
pub struct HybridObservationBatch<B: Backend> {
    /// The visual pixel data. Shape: `[batch_size, frame_stack, height, width]`.
    pub frames: Tensor<B, 4>,
    /// The numeric RAM-extracted features. Shape: `[batch_size, feature_count]`.
    pub features: Tensor<B, 2>,
}

/// The paired outputs returned from a forward pass of the hybrid network.
#[derive(Debug)]
pub struct HybridPolicyValueOutput<B: Backend> {
    /// The unnormalized action probabilities. Shape: `[batch_size, action_count]`.
    pub policy_logits: Tensor<B, 2>,
    /// The estimated future reward value. Shape: `[batch_size, 1]`.
    pub value: Tensor<B, 2>,
}

/// The core neural network architecture combining CNN vision and dense feature processing.
#[derive(Module, Debug)]
pub struct HybridPolicyValueNet<B: Backend> {
    conv1: Conv2d<B>,
    conv2: Conv2d<B>,
    pool: AdaptiveAvgPool2d,
    activation: Relu,
    feature_proj: Linear<B>,
    trunk: Linear<B>,
    policy_head: Linear<B>,
    value_head: Linear<B>,
}

impl<B: Backend> HybridPolicyValueNet<B> {
    /// Constructs the layers of the hybrid network.
    #[must_use]
    pub fn new(device: &B::Device, cfg: &HybridPolicyValueConfig) -> Self {
        let conv1 = Conv2dConfig::new([cfg.frame_stack, 16], [8, 8])
            .with_stride([4, 4])
            .init(device);
        let conv2 = Conv2dConfig::new([16, 32], [4, 4])
            .with_stride([2, 2])
            .init(device);
        let pool = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let feature_proj = LinearConfig::new(cfg.feature_count, 32).init(device);
        let trunk = LinearConfig::new(64, 64).init(device);
        let policy_head = LinearConfig::new(64, cfg.action_count).init(device);
        let value_head = LinearConfig::new(64, 1).init(device);

        Self {
            conv1,
            conv2,
            pool,
            activation: Relu::new(),
            feature_proj,
            trunk,
            policy_head,
            value_head,
        }
    }

    /// Executes a forward pass of the network, transforming an observation batch
    /// into action probabilities (policy) and a state estimate (value).
    #[must_use]
    pub fn forward(&self, batch: HybridObservationBatch<B>) -> HybridPolicyValueOutput<B> {
        let vision = self.activation.forward(self.conv1.forward(batch.frames));
        let vision = self.activation.forward(self.conv2.forward(vision));
        let vision: Tensor<B, 2> = self.pool.forward(vision).flatten(1, 3);

        let features = self
            .activation
            .forward(self.feature_proj.forward(batch.features));
        let joined = Tensor::cat(vec![vision, features], 1);
        let trunk = self.activation.forward(self.trunk.forward(joined));

        HybridPolicyValueOutput {
            policy_logits: self.policy_head.forward(trunk.clone()),
            value: self.value_head.forward(trunk),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::ObservationSnapshot;

    #[test]
    fn should_return_config_from_observation() {
        let obs = ObservationSnapshot {
            frame_stack: 4,
            width: 84,
            height: 84,
            frames: vec![],
            features: vec![0.0; 12],
        };

        let config = HybridPolicyValueConfig::from_observation(&obs, 14);

        assert_eq!(config.frame_stack, 4, "Frame stack mismatch");
        assert_eq!(config.feature_count, 12, "Feature count mismatch");
        assert_eq!(config.action_count, 14, "Action count mismatch");
        assert_eq!(config.observation_width, 84, "Width mismatch");
        assert_eq!(config.observation_height, 84, "Height mismatch");
    }
}
