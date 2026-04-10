//! ML Model Architectures.
//!
//! This module defines the neural network architecture used by our reinforcement
//! learning agents. It leverages the `burn` deep learning framework.

use burn_core as burn;
use burn_core::prelude::{Backend, Config, Module, Tensor};
use burn_nn::{
    Linear, LinearConfig, Relu,
    conv::{Conv2d, Conv2dConfig},
    pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig},
};

use crate::{config::MIN_OBSERVATION_DIM, env::ObservationSnapshot};

/// Configuration for initializing the [`HybridPolicyValueNet`].
#[derive(Config, Debug)]
pub struct HybridPolicyValueConfig {
    /// Number of consecutive frames stacked into the visual input channel.
    pub frame_stack: usize,
    /// Number of distinct scalar features extracted from the emulator state.
    pub feature_count: usize,
    /// The size of the discrete action space.
    pub action_count: usize,
    /// Expected width of the visual observation tensor.
    #[config(default = "84")]
    pub observation_width: usize,
    /// Expected height of the visual observation tensor.
    #[config(default = "84")]
    pub observation_height: usize,
}

impl HybridPolicyValueConfig {
    /// Creates a configuration dynamically matched to an observation's dimensions.
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

    /// Initializes a new instance of the model on the specified device.
    ///
    /// # Panics
    ///
    /// Panics if the configuration specifies a dimension size of zero.
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

/// A batch of observations prepared for the forward pass.
#[derive(Debug)]
pub struct HybridObservationBatch<B: Backend> {
    /// Visual frames tensor of shape `[batch_size, frame_stack, height, width]`.
    pub frames: Tensor<B, 4>,
    /// Scalar features tensor of shape `[batch_size, feature_count]`.
    pub features: Tensor<B, 2>,
}

/// The output of a forward pass through the model.
#[derive(Debug)]
pub struct HybridPolicyValueOutput<B: Backend> {
    /// Unnormalized action probabilities.
    pub policy_logits: Tensor<B, 2>,
    /// Estimated state value for calculating advantage.
    pub value: Tensor<B, 2>,
}

/// A neural network architecture that combines visual and scalar inputs.
///
/// This network uses convolutional layers to process the frame stack and linear
/// layers to process the game-specific scalar features (like speed, position).
/// The two streams are concatenated and fed into a common trunk, which then splits
/// into an actor (policy) head and a critic (value) head.
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
    /// Instantiates the network modules using the provided configuration.
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

    /// Executes the forward pass of the model, mapping observations to logits and values.
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
