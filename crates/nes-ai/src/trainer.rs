use burn_autodiff::Autodiff;
use burn_ndarray::NdArray;

use crate::{actions::ControlAction, error::AiError};

pub type TrainBackend = Autodiff<NdArray<f32>>;

#[derive(Debug, Clone)]
pub struct TrainerConfig {
    pub seed: u64,
    pub rollout_steps: usize,
    pub minibatch_size: usize,
    pub epochs: usize,
    pub learning_rate: f64,
}

impl TrainerConfig {
    #[must_use]
    pub fn smoke() -> Self {
        Self {
            seed: 7,
            rollout_steps: 32,
            minibatch_size: 16,
            epochs: 2,
            learning_rate: 1e-3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EvalSummary {
    pub average_return: f32,
}

/// Evaluates a random policy against the mock smoke environment.
///
/// # Errors
///
/// Returns [`AiError::Unsupported`] when `episodes` is zero or the trainer
/// configuration is internally inconsistent.
pub fn evaluate_random_policy(
    cfg: &TrainerConfig,
    episodes: usize,
) -> Result<EvalSummary, AiError> {
    validate_request(cfg, episodes)?;
    Ok(EvalSummary {
        average_return: mock_baseline_return(cfg, episodes),
    })
}

/// Runs a deterministic mock PPO pass against the smoke environment.
///
/// # Errors
///
/// Returns [`AiError::Unsupported`] when `episodes` is zero or the trainer
/// configuration is internally inconsistent.
pub fn run_mock_ppo_smoke(cfg: &TrainerConfig, episodes: usize) -> Result<EvalSummary, AiError> {
    validate_request(cfg, episodes)?;
    let baseline = mock_baseline_return(cfg, episodes);

    Ok(EvalSummary {
        average_return: baseline + mock_training_lift(cfg),
    })
}

/// Parses a CLI action label into the discrete control action space.
///
/// # Errors
///
/// Returns [`AiError::Unsupported`] when `value` does not map to a known
/// control action label.
pub fn action_from_arg(value: &str) -> Result<ControlAction, AiError> {
    match value {
        "noop" => Ok(ControlAction::Noop),
        "right" => Ok(ControlAction::Right),
        "right-a" => Ok(ControlAction::RightA),
        "a" => Ok(ControlAction::A),
        "right-b" => Ok(ControlAction::RightB),
        "right-a-b" => Ok(ControlAction::RightAB),
        _ => Err(AiError::Unsupported("unknown action")),
    }
}

fn validate_request(cfg: &TrainerConfig, episodes: usize) -> Result<(), AiError> {
    if episodes == 0 {
        return Err(AiError::Unsupported("episodes must be greater than zero"));
    }
    if cfg.rollout_steps == 0 {
        return Err(AiError::Unsupported(
            "rollout_steps must be greater than zero",
        ));
    }
    if cfg.minibatch_size == 0 {
        return Err(AiError::Unsupported(
            "minibatch_size must be greater than zero",
        ));
    }
    if cfg.learning_rate <= 0.0 {
        return Err(AiError::Unsupported(
            "learning_rate must be greater than zero",
        ));
    }
    Ok(())
}

fn mock_baseline_return(cfg: &TrainerConfig, episodes: usize) -> f32 {
    let episode_confidence = 1.0 - 1.0 / (usize_to_f32(episodes) + 1.0);
    let rollout_ratio = usize_to_f32(cfg.rollout_steps) / usize_to_f32(cfg.minibatch_size);
    let seed_bias = small_u64_to_f32(cfg.seed % 7) * 0.001;

    0.05 * episode_confidence + 0.01 * rollout_ratio + seed_bias
}

fn mock_training_lift(cfg: &TrainerConfig) -> f32 {
    let epoch_gain = usize_to_f32(cfg.epochs) * 0.05;
    let rollout_gain = (usize_to_f32(cfg.rollout_steps) / usize_to_f32(cfg.minibatch_size)) * 0.01;

    epoch_gain + rollout_gain
}

fn usize_to_f32(value: usize) -> f32 {
    let clamped = value.min(usize::from(u16::MAX));
    let clamped = u16::try_from(clamped).unwrap_or(u16::MAX);

    f32::from(clamped)
}

fn small_u64_to_f32(value: u64) -> f32 {
    let clamped = u8::try_from(value).unwrap_or(u8::MAX);

    f32::from(clamped)
}
