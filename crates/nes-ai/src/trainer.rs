use std::path::{Path, PathBuf};

use burn_autodiff::Autodiff;
use burn_core::{
    module::{AutodiffModule, Module, ModuleMapper, Param},
    record::{DefaultRecorder, FileRecorder},
};
use burn_ndarray::NdArray;
use burn_tensor::{
    Int, Tensor, TensorData,
    activation::{log_softmax, softmax},
    backend::{AutodiffBackend, Backend},
};
use nes_core::tas::{TasMovie, TasRecorder};
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};

use crate::{
    actions::ControlAction,
    config::{AiProfileConfig, GameProfileId},
    env::{AnyControlEnv, ObservationSnapshot, ProfileEnv},
    episode::{EpisodeArtifactPaths, EpisodeArtifactWriter, EpisodeMetadata},
    error::AiError,
    model::{HybridObservationBatch, HybridPolicyValueConfig, HybridPolicyValueNet},
    obs::FrameStack,
    profile::TaskProfile,
    reward::RewardFeatures,
};

/// The Burn backend type used during training (NdArray with Autodiff).
pub type TrainBackend = Autodiff<NdArray<f32>>;
type InferBackend = NdArray<f32>;

const SMOKE_FRAME_STACK: usize = 4;
const SMOKE_OBSERVATION_WIDTH: usize = 20;
const SMOKE_OBSERVATION_HEIGHT: usize = 20;
const SMOKE_MAX_EPISODE_FRAMES: u32 = 60;
const SMOKE_ACTION_GAINS: [f32; ControlAction::action_count()] = [0.0, 0.05, 0.10, 0.0, 0.25, 1.25];

/// Configuration hyperparameters for the PPO Trainer.
///
/// # Examples
///
/// ```
/// use nes_ai::trainer::TrainerConfig;
///
/// let config = TrainerConfig::smoke();
/// assert_eq!(config.rollout_steps, 16);
/// ```
#[derive(Debug, Clone)]
pub struct TrainerConfig {
    /// Initial random seed for reproducibility.
    pub seed: u64,
    /// Number of steps to collect per rollout phase.
    pub rollout_steps: usize,
    /// Batch size used during the PPO update epoch.
    pub minibatch_size: usize,
    /// Number of PPO update epochs per rollout.
    pub epochs: usize,
    /// Total number of rollout/update cycles to run.
    pub training_updates: usize,
    /// Step size for gradient descent.
    pub learning_rate: f32,
    /// Discount factor (gamma) for future rewards.
    pub discount_gamma: f32,
    /// GAE lambda parameter for advantage smoothing.
    pub gae_lambda: f32,
    /// PPO clipping threshold.
    pub clip_epsilon: f32,
    /// Weight of the entropy bonus to encourage exploration.
    pub entropy_coefficient: f32,
    /// Weight of the value function loss.
    pub value_loss_coefficient: f32,
    /// How often (in updates) to save model checkpoints.
    pub checkpoint_interval: usize,
    /// Optional directory to save `.json.gz` checkpoint files.
    pub checkpoint_dir: Option<PathBuf>,
    /// Optional directory to save `.tas.json` and `.run.json` artifacts.
    pub artifact_dir: Option<PathBuf>,
}

impl TrainerConfig {
    /// Returns a highly constrained configuration suitable for fast unit testing.
    #[must_use]
    pub fn smoke() -> Self {
        Self {
            seed: 7,
            rollout_steps: 16,
            minibatch_size: 8,
            epochs: 1,
            training_updates: 2,
            learning_rate: 5e-2,
            discount_gamma: 0.99,
            gae_lambda: 0.95,
            clip_epsilon: 0.2,
            entropy_coefficient: 0.01,
            value_loss_coefficient: 0.5,
            checkpoint_interval: 1,
            checkpoint_dir: None,
            artifact_dir: None,
        }
    }
}

/// Summary metrics emitted after an evaluation phase.
#[derive(Debug, Clone)]
pub struct EvalSummary {
    /// The mean total reward accumulated across all evaluated episodes.
    pub average_return: f32,
    /// The file paths of any artifacts (movies, JSON) generated during evaluation.
    pub artifact_paths: Vec<EpisodeArtifactPaths>,
}

/// Summary metrics emitted after a complete training run.
#[derive(Debug, Clone)]
pub struct TrainSummary {
    /// The mean total reward accumulated during the final rollout phase.
    pub average_return: f32,
    /// The file paths of all ML checkpoints saved during the run.
    pub checkpoint_paths: Vec<PathBuf>,
    /// The file paths of any artifacts (movies, JSON) generated during training.
    pub artifact_paths: Vec<EpisodeArtifactPaths>,
}

#[derive(Debug, Clone)]
struct TrainerStep {
    observation: ObservationSnapshot,
    reward_total: f32,
    done: bool,
}

#[derive(Debug, Clone)]
struct RolloutTransition {
    observation: ObservationSnapshot,
    action_index: usize,
    reward: f32,
    done: bool,
    value: f32,
    log_prob: f32,
}

#[derive(Debug, Clone)]
struct PpoSample {
    observation: ObservationSnapshot,
    action_index: usize,
    old_log_prob: f32,
    advantage: f32,
    return_estimate: f32,
}

#[derive(Debug)]
struct TrainBatch<B: Backend> {
    observations: HybridObservationBatch<B>,
    action_indices: Tensor<B, 2, Int>,
    old_log_probs: Tensor<B, 2>,
    advantages: Tensor<B, 2>,
    returns: Tensor<B, 2>,
}

#[derive(Debug, Clone, Copy)]
struct PolicyDecision {
    action_index: usize,
    log_prob: f32,
    value: f32,
}

trait TrainerEnv {
    fn reset_env(&mut self) -> Result<ObservationSnapshot, AiError>;
    fn step_env(&mut self, action: ControlAction) -> Result<TrainerStep, AiError>;
    fn finish_episode_meta(&self, total_reward: f32) -> EpisodeMetadata;
    fn recorded_movie(&self) -> &TasMovie;
}

impl<P> TrainerEnv for ProfileEnv<P>
where
    P: TaskProfile,
    P::Features: RewardFeatures,
{
    fn reset_env(&mut self) -> Result<ObservationSnapshot, AiError> {
        self.reset()?;
        self.observation()
    }

    fn step_env(&mut self, action: ControlAction) -> Result<TrainerStep, AiError> {
        let step = self.step(action)?;
        Ok(TrainerStep {
            observation: self.observation()?,
            reward_total: step.reward.total,
            done: step.done,
        })
    }

    fn finish_episode_meta(&self, total_reward: f32) -> EpisodeMetadata {
        self.finish_episode(total_reward)
    }

    fn recorded_movie(&self) -> &TasMovie {
        self.recorded_movie()
    }
}

impl TrainerEnv for AnyControlEnv {
    fn reset_env(&mut self) -> Result<ObservationSnapshot, AiError> {
        self.reset()?;
        self.observation()
    }

    fn step_env(&mut self, action: ControlAction) -> Result<TrainerStep, AiError> {
        let step = self.step(action)?;
        Ok(TrainerStep {
            observation: self.observation()?,
            reward_total: step.reward.total,
            done: step.done,
        })
    }

    fn finish_episode_meta(&self, total_reward: f32) -> EpisodeMetadata {
        AnyControlEnv::finish_episode(self, total_reward)
    }

    fn recorded_movie(&self) -> &TasMovie {
        AnyControlEnv::recorded_movie(self)
    }
}

#[derive(Debug, Clone)]
struct SmokeEnv {
    recorder: TasRecorder,
    frame_stack: FrameStack,
    episode_frames: u32,
    done: bool,
    position: f32,
    velocity: f32,
    last_gain: f32,
}

impl SmokeEnv {
    fn new() -> Self {
        Self {
            recorder: TasRecorder::new(),
            frame_stack: FrameStack::new(
                SMOKE_FRAME_STACK,
                SMOKE_OBSERVATION_WIDTH * SMOKE_OBSERVATION_HEIGHT,
            ),
            episode_frames: 0,
            done: false,
            position: 0.0,
            velocity: 0.0,
            last_gain: 0.0,
        }
    }

    fn observation(&self) -> ObservationSnapshot {
        ObservationSnapshot {
            frame_stack: SMOKE_FRAME_STACK,
            width: SMOKE_OBSERVATION_WIDTH,
            height: SMOKE_OBSERVATION_HEIGHT,
            frames: self.frame_stack.flattened(),
            features: vec![
                self.position / 64.0,
                self.velocity / 4.0,
                self.last_gain,
                u32_to_f32(self.episode_frames) / u32_to_f32(SMOKE_MAX_EPISODE_FRAMES),
                if self.done { 1.0 } else { 0.0 },
                1.0,
            ],
        }
    }

    fn push_rendered_frame(&mut self) {
        self.frame_stack.push(self.render_frame());
    }

    fn render_frame(&self) -> Vec<f32> {
        let mut frame = vec![0.0; SMOKE_OBSERVATION_WIDTH * SMOKE_OBSERVATION_HEIGHT];
        let column = quantize_to_index(self.position, SMOKE_OBSERVATION_WIDTH - 1);
        for row in 0..SMOKE_OBSERVATION_HEIGHT {
            frame[row * SMOKE_OBSERVATION_WIDTH + column] = 1.0;
        }

        let velocity_column =
            quantize_to_index(self.velocity.abs() * 8.0, SMOKE_OBSERVATION_WIDTH - 1);
        frame[(SMOKE_OBSERVATION_HEIGHT - 1) * SMOKE_OBSERVATION_WIDTH + velocity_column] = 0.5;
        frame
    }

    fn final_state_hash(&self) -> u64 {
        u64::from(self.episode_frames)
            ^ u64::from(self.position.to_bits())
            ^ (u64::from(self.velocity.to_bits()) << 1)
    }
}

impl TrainerEnv for SmokeEnv {
    fn reset_env(&mut self) -> Result<ObservationSnapshot, AiError> {
        self.recorder = TasRecorder::new();
        self.recorder.start();
        self.frame_stack = FrameStack::new(
            SMOKE_FRAME_STACK,
            SMOKE_OBSERVATION_WIDTH * SMOKE_OBSERVATION_HEIGHT,
        );
        self.episode_frames = 0;
        self.done = false;
        self.position = 0.0;
        self.velocity = 0.0;
        self.last_gain = 0.0;
        for _ in 0..SMOKE_FRAME_STACK {
            self.push_rendered_frame();
        }
        Ok(self.observation())
    }

    fn step_env(&mut self, action: ControlAction) -> Result<TrainerStep, AiError> {
        if self.done {
            return Err(AiError::Unsupported("step after episode done"));
        }

        self.recorder.record_frame(action.controller1_bits());
        self.episode_frames = self.episode_frames.saturating_add(1);
        let gain = SMOKE_ACTION_GAINS[action_index(action)];
        self.velocity = (self.velocity * 0.35) + gain;
        self.position += self.velocity;
        self.last_gain = gain;
        self.done = self.episode_frames >= SMOKE_MAX_EPISODE_FRAMES;
        self.push_rendered_frame();

        let reward = if matches!(action, ControlAction::Noop) {
            -0.15
        } else {
            gain
        };

        Ok(TrainerStep {
            observation: self.observation(),
            reward_total: reward,
            done: self.done,
        })
    }

    fn finish_episode_meta(&self, total_reward: f32) -> EpisodeMetadata {
        EpisodeMetadata {
            profile_id: "smoke-control".to_owned(),
            snapshot_id: "smoke-v1".to_owned(),
            rom_hash: "mock".to_owned(),
            total_reward,
            episode_frames: u64::from(self.episode_frames),
            final_state_hash: self.final_state_hash(),
        }
    }

    fn recorded_movie(&self) -> &TasMovie {
        self.recorder.movie()
    }
}

/// Evaluates a uniform random policy against the smoke environment.
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
    evaluate_random_with_factory(cfg, episodes, || Ok(SmokeEnv::new()))
}

/// Runs PPO training against the deterministic smoke environment.
///
/// # Errors
///
/// Returns [`AiError`] if the trainer configuration is invalid, checkpoint
/// writing fails, or evaluation artifacts cannot be emitted.
pub fn run_mock_ppo_smoke(cfg: &TrainerConfig, episodes: usize) -> Result<TrainSummary, AiError> {
    validate_request(cfg, episodes)?;
    train_with_factory(cfg, episodes, || Ok(SmokeEnv::new()))
}

/// Runs PPO training against the SMB control profile.
///
/// # Errors
///
/// Returns [`AiError`] if the profile cannot be loaded, training fails, or
/// checkpoint/artifact emission fails.
pub fn train_control_profile(
    profile_cfg: &AiProfileConfig,
    cfg: &TrainerConfig,
    episodes: usize,
) -> Result<TrainSummary, AiError> {
    validate_request(cfg, episodes)?;
    let profile_cfg = profile_cfg.clone();
    train_with_factory(cfg, episodes, move || {
        AnyControlEnv::from_config(profile_cfg.clone())
    })
}

/// Runs PPO training against the SMB control profile.
///
/// # Errors
///
/// Returns [`AiError`] if `profile_cfg` is not an SMB profile, the profile
/// cannot be loaded, training fails, or checkpoint/artifact emission fails.
pub fn train_smb_control(
    profile_cfg: &AiProfileConfig,
    cfg: &TrainerConfig,
    episodes: usize,
) -> Result<TrainSummary, AiError> {
    if profile_cfg.game != GameProfileId::Smb {
        return Err(AiError::Unsupported(
            "train_smb_control requires game = \"smb\"",
        ));
    }
    train_control_profile(profile_cfg, cfg, episodes)
}

/// Evaluates a configured control profile either from a saved checkpoint or with a random baseline.
///
/// # Errors
///
/// Returns [`AiError`] if the profile cannot be loaded, the checkpoint cannot be
/// loaded, or evaluation artifacts cannot be emitted.
pub fn evaluate_control_profile(
    profile_cfg: &AiProfileConfig,
    cfg: &TrainerConfig,
    episodes: usize,
    checkpoint_base: Option<&Path>,
) -> Result<EvalSummary, AiError> {
    validate_request(cfg, episodes)?;
    let profile_cfg_for_eval = profile_cfg.clone();
    let make_env = move || AnyControlEnv::from_config(profile_cfg_for_eval.clone());

    match checkpoint_base {
        Some(checkpoint_base) => {
            let mut env = make_env()?;
            let observation = env.reset_env()?;
            let model_cfg = HybridPolicyValueConfig::from_observation(
                &observation,
                ControlAction::action_count(),
            );
            let device = <InferBackend as Backend>::Device::default();
            let recorder = DefaultRecorder::new();
            let checkpoint_base = checkpoint_base_path(checkpoint_base);
            let model = model_cfg
                .init::<InferBackend>(&device)
                .load_file(checkpoint_base.clone(), &recorder, &device)
                .map_err(|source| AiError::CheckpointLoad {
                    path: checkpoint_file_path(&checkpoint_base),
                    source,
                })?;
            evaluate_model_with_factory(cfg, episodes, &model, make_env)
        }
        None => evaluate_random_with_factory(cfg, episodes, make_env),
    }
}

/// Evaluates SMB control either from a saved checkpoint or with a random baseline.
///
/// # Errors
///
/// Returns [`AiError`] if `profile_cfg` is not an SMB profile, the profile
/// cannot be loaded, the checkpoint cannot be loaded, or evaluation artifacts
/// cannot be emitted.
pub fn evaluate_smb_control(
    profile_cfg: &AiProfileConfig,
    cfg: &TrainerConfig,
    episodes: usize,
    checkpoint_base: Option<&Path>,
) -> Result<EvalSummary, AiError> {
    if profile_cfg.game != GameProfileId::Smb {
        return Err(AiError::Unsupported(
            "evaluate_smb_control requires game = \"smb\"",
        ));
    }
    evaluate_control_profile(profile_cfg, cfg, episodes, checkpoint_base)
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

fn train_with_factory<E, F>(
    cfg: &TrainerConfig,
    episodes: usize,
    make_env: F,
) -> Result<TrainSummary, AiError>
where
    E: TrainerEnv,
    F: Fn() -> Result<E, AiError>,
{
    let device = <TrainBackend as Backend>::Device::default();
    TrainBackend::seed(&device, cfg.seed);
    let mut rng = StdRng::seed_from_u64(cfg.seed);
    let mut env = make_env()?;
    let initial_observation = env.reset_env()?;
    let model_cfg = HybridPolicyValueConfig::from_observation(
        &initial_observation,
        ControlAction::action_count(),
    );
    let mut model = Some(model_cfg.init::<TrainBackend>(&device));
    let mut checkpoint_paths = Vec::new();

    for update in 0..cfg.training_updates {
        let rollout = collect_rollout(cfg, &mut env, model.as_ref().unwrap(), &mut rng)?;
        ppo_update(cfg, &mut model, &rollout, &mut rng);

        if cfg
            .checkpoint_dir
            .as_ref()
            .is_some_and(|_| (update + 1) % cfg.checkpoint_interval == 0)
        {
            checkpoint_paths.push(save_checkpoint(model.as_ref().unwrap(), cfg, update + 1)?);
        }
    }

    let infer_model = model.as_ref().unwrap().valid();
    let evaluation = evaluate_model_with_factory(cfg, episodes, &infer_model, make_env)?;

    Ok(TrainSummary {
        average_return: evaluation.average_return,
        checkpoint_paths,
        artifact_paths: evaluation.artifact_paths,
    })
}

fn evaluate_random_with_factory<E, F>(
    cfg: &TrainerConfig,
    episodes: usize,
    make_env: F,
) -> Result<EvalSummary, AiError>
where
    E: TrainerEnv,
    F: Fn() -> Result<E, AiError>,
{
    let mut rng = StdRng::seed_from_u64(cfg.seed ^ 0x5EED_F00D);
    let writer = cfg.artifact_dir.clone().map(EpisodeArtifactWriter::new);
    let mut env = make_env()?;
    let mut returns = Vec::with_capacity(episodes);
    let mut artifact_paths = Vec::new();

    for episode_index in 0..episodes {
        let mut total_reward = 0.0;
        let _ = env.reset_env()?;
        loop {
            let action = action_from_index(rng.gen_range(0..ControlAction::action_count()));
            let step = env.step_env(action)?;
            total_reward += step.reward_total;
            if step.done {
                break;
            }
        }

        returns.push(total_reward);
        if let Some(writer) = &writer {
            let prefix = format!("eval-ep{episode_index:04}");
            let meta = env.finish_episode_meta(total_reward);
            artifact_paths.push(writer.write(&prefix, env.recorded_movie(), &meta)?);
        }
    }

    Ok(EvalSummary {
        average_return: mean_f32(&returns),
        artifact_paths,
    })
}

fn evaluate_model_with_factory<E, F>(
    cfg: &TrainerConfig,
    episodes: usize,
    model: &HybridPolicyValueNet<InferBackend>,
    make_env: F,
) -> Result<EvalSummary, AiError>
where
    E: TrainerEnv,
    F: Fn() -> Result<E, AiError>,
{
    let writer = cfg.artifact_dir.clone().map(EpisodeArtifactWriter::new);
    let mut env = make_env()?;
    let mut returns = Vec::with_capacity(episodes);
    let mut artifact_paths = Vec::new();

    for episode_index in 0..episodes {
        let mut total_reward = 0.0;
        let mut observation = env.reset_env()?;
        loop {
            let decision = greedy_policy_decision(model, &observation);
            let step = env.step_env(action_from_index(decision.action_index))?;
            total_reward += step.reward_total;
            if step.done {
                break;
            }
            observation = step.observation;
        }

        returns.push(total_reward);
        if let Some(writer) = &writer {
            let prefix = format!("eval-ep{episode_index:04}");
            let meta = env.finish_episode_meta(total_reward);
            artifact_paths.push(writer.write(&prefix, env.recorded_movie(), &meta)?);
        }
    }

    Ok(EvalSummary {
        average_return: mean_f32(&returns),
        artifact_paths,
    })
}

fn collect_rollout<E: TrainerEnv>(
    cfg: &TrainerConfig,
    env: &mut E,
    model: &HybridPolicyValueNet<TrainBackend>,
    rng: &mut StdRng,
) -> Result<Vec<PpoSample>, AiError> {
    let infer_model = model.valid();
    let mut observation = env.reset_env()?;
    let mut transitions = Vec::with_capacity(cfg.rollout_steps);

    for _ in 0..cfg.rollout_steps {
        let decision = sampled_policy_decision(&infer_model, &observation, rng);
        let step = env.step_env(action_from_index(decision.action_index))?;
        transitions.push(RolloutTransition {
            observation: observation.clone(),
            action_index: decision.action_index,
            reward: step.reward_total,
            done: step.done,
            value: decision.value,
            log_prob: decision.log_prob,
        });

        observation = if step.done {
            env.reset_env()?
        } else {
            step.observation
        };
    }

    let bootstrap_value = transitions.last().map_or(0.0, |transition| {
        if transition.done {
            0.0
        } else {
            greedy_policy_decision(&infer_model, &observation).value
        }
    });

    let (advantages, returns) = compute_gae(
        &transitions,
        bootstrap_value,
        cfg.discount_gamma,
        cfg.gae_lambda,
    );
    let mut samples = transitions
        .into_iter()
        .zip(advantages)
        .zip(returns)
        .map(|((transition, advantage), return_estimate)| PpoSample {
            observation: transition.observation,
            action_index: transition.action_index,
            old_log_prob: transition.log_prob,
            advantage,
            return_estimate,
        })
        .collect::<Vec<_>>();
    normalize_advantages(&mut samples);

    Ok(samples)
}

fn compute_gae(
    transitions: &[RolloutTransition],
    bootstrap_value: f32,
    gamma: f32,
    lambda: f32,
) -> (Vec<f32>, Vec<f32>) {
    let mut advantages = vec![0.0; transitions.len()];
    let mut returns = vec![0.0; transitions.len()];
    let mut gae = 0.0;

    for index in (0..transitions.len()).rev() {
        let non_terminal = if transitions[index].done { 0.0 } else { 1.0 };
        let next_value = if index + 1 == transitions.len() {
            bootstrap_value
        } else {
            transitions[index + 1].value
        };
        let delta = transitions[index].reward + gamma * next_value * non_terminal
            - transitions[index].value;
        gae = delta + gamma * lambda * non_terminal * gae;
        advantages[index] = gae;
        returns[index] = gae + transitions[index].value;
    }

    (advantages, returns)
}

fn normalize_advantages(samples: &mut [PpoSample]) {
    if samples.is_empty() {
        return;
    }

    let count = usize_to_f32(samples.len());
    let mean = samples.iter().map(|sample| sample.advantage).sum::<f32>() / count;
    let variance = samples
        .iter()
        .map(|sample| {
            let centered = sample.advantage - mean;
            centered * centered
        })
        .sum::<f32>()
        / count;
    let stddev = variance.sqrt();
    if stddev <= f32::EPSILON {
        return;
    }

    for sample in samples {
        sample.advantage = (sample.advantage - mean) / stddev;
    }
}

fn ppo_update(
    cfg: &TrainerConfig,
    model: &mut Option<HybridPolicyValueNet<TrainBackend>>,
    samples: &[PpoSample],
    rng: &mut StdRng,
) {
    let device = <TrainBackend as Backend>::Device::default();
    let mut indices = (0..samples.len()).collect::<Vec<_>>();

    for _ in 0..cfg.epochs {
        indices.shuffle(rng);
        for chunk in indices.chunks(cfg.minibatch_size) {
            let batch = build_train_batch(samples, chunk, &device);
            let output = model.as_ref().unwrap().forward(batch.observations);
            let log_probs = log_softmax(output.policy_logits.clone(), 1);
            let selected_log_probs = log_probs.clone().gather(1, batch.action_indices);
            let ratios = (selected_log_probs.clone() - batch.old_log_probs).exp();
            let unclipped = ratios.clone() * batch.advantages.clone();
            let clipped =
                ratios.clamp(1.0 - cfg.clip_epsilon, 1.0 + cfg.clip_epsilon) * batch.advantages;
            let policy_loss = unclipped.min_pair(clipped).mean().mul_scalar(-1.0);
            let value_loss = (output.value.clone() - batch.returns)
                .powf_scalar(2.0)
                .mean();
            let entropy = (softmax(output.policy_logits, 1) * log_probs)
                .sum_dim(1)
                .mean()
                .mul_scalar(-1.0);
            let loss = policy_loss + value_loss.mul_scalar(cfg.value_loss_coefficient)
                - entropy.mul_scalar(cfg.entropy_coefficient);
            let mut grads = loss.backward();
            let mut optimizer = SgdStep {
                grads: &mut grads,
                learning_rate: cfg.learning_rate,
            };
            let current = model.take().expect("model should exist during PPO update");
            *model = Some(current.map(&mut optimizer));
        }
    }
}

fn build_train_batch<B: Backend>(
    samples: &[PpoSample],
    indices: &[usize],
    device: &B::Device,
) -> TrainBatch<B> {
    let first = &samples[indices[0]].observation;
    let mut frames =
        Vec::with_capacity(indices.len() * first.frame_stack * first.width * first.height);
    let mut features = Vec::with_capacity(indices.len() * first.features.len());
    let mut action_indices = Vec::with_capacity(indices.len());
    let mut old_log_probs = Vec::with_capacity(indices.len());
    let mut advantages = Vec::with_capacity(indices.len());
    let mut returns = Vec::with_capacity(indices.len());

    for &index in indices {
        let sample = &samples[index];
        frames.extend_from_slice(&sample.observation.frames);
        features.extend_from_slice(&sample.observation.features);
        action_indices.push(i64::try_from(sample.action_index).unwrap_or(0));
        old_log_probs.push(sample.old_log_prob);
        advantages.push(sample.advantage);
        returns.push(sample.return_estimate);
    }

    let batch_size = indices.len();
    TrainBatch {
        observations: HybridObservationBatch {
            frames: Tensor::<B, 4>::from_data(
                TensorData::new(
                    frames,
                    [batch_size, first.frame_stack, first.height, first.width],
                ),
                device,
            ),
            features: Tensor::<B, 2>::from_data(
                TensorData::new(features, [batch_size, first.features.len()]),
                device,
            ),
        },
        action_indices: Tensor::<B, 2, Int>::from_data(
            TensorData::new(action_indices, [batch_size, 1]),
            device,
        ),
        old_log_probs: Tensor::<B, 2>::from_data(
            TensorData::new(old_log_probs, [batch_size, 1]),
            device,
        ),
        advantages: Tensor::<B, 2>::from_data(TensorData::new(advantages, [batch_size, 1]), device),
        returns: Tensor::<B, 2>::from_data(TensorData::new(returns, [batch_size, 1]), device),
    }
}

fn save_checkpoint(
    model: &HybridPolicyValueNet<TrainBackend>,
    cfg: &TrainerConfig,
    update_index: usize,
) -> Result<PathBuf, AiError> {
    let Some(checkpoint_dir) = &cfg.checkpoint_dir else {
        return Err(AiError::Unsupported("checkpoint dir missing"));
    };

    let base_path = checkpoint_dir.join(format!("policy-update-{update_index:04}"));
    let recorder = DefaultRecorder::new();
    model
        .valid()
        .save_file(base_path.clone(), &recorder)
        .map_err(|source| AiError::CheckpointSave {
            path: checkpoint_file_path(&base_path),
            source,
        })?;
    Ok(base_path)
}

fn checkpoint_file_path(base_path: &Path) -> PathBuf {
    let mut path = base_path.to_path_buf();
    path.set_extension(<DefaultRecorder as FileRecorder<InferBackend>>::file_extension());
    path
}

fn checkpoint_base_path(path: &Path) -> PathBuf {
    let expected_extension = <DefaultRecorder as FileRecorder<InferBackend>>::file_extension();
    if path
        .extension()
        .is_some_and(|extension| extension == expected_extension)
    {
        let mut base = path.to_path_buf();
        let _ = base.set_extension("");
        return base;
    }
    path.to_path_buf()
}

fn greedy_policy_decision(
    model: &HybridPolicyValueNet<InferBackend>,
    observation: &ObservationSnapshot,
) -> PolicyDecision {
    let (logits, value) = forward_policy(model, observation);
    let probabilities = logits_to_probabilities(&logits);
    let action_index = argmax_index(&logits);

    PolicyDecision {
        action_index,
        log_prob: probabilities[action_index].max(f32::MIN_POSITIVE).ln(),
        value,
    }
}

fn sampled_policy_decision(
    model: &HybridPolicyValueNet<InferBackend>,
    observation: &ObservationSnapshot,
    rng: &mut StdRng,
) -> PolicyDecision {
    let (logits, value) = forward_policy(model, observation);
    let probabilities = logits_to_probabilities(&logits);
    let action_index = sample_from_probabilities(&probabilities, rng);

    PolicyDecision {
        action_index,
        log_prob: probabilities[action_index].max(f32::MIN_POSITIVE).ln(),
        value,
    }
}

fn forward_policy(
    model: &HybridPolicyValueNet<InferBackend>,
    observation: &ObservationSnapshot,
) -> (Vec<f32>, f32) {
    let device = <InferBackend as Backend>::Device::default();
    let batch = HybridObservationBatch {
        frames: Tensor::<InferBackend, 4>::from_data(
            TensorData::new(
                observation.frames.clone(),
                [
                    1,
                    observation.frame_stack,
                    observation.height,
                    observation.width,
                ],
            ),
            &device,
        ),
        features: Tensor::<InferBackend, 2>::from_data(
            TensorData::new(
                observation.features.clone(),
                [1, observation.features.len()],
            ),
            &device,
        ),
    };
    let output = model.forward(batch);
    let logits = output
        .policy_logits
        .into_data()
        .as_slice::<f32>()
        .map_or_else(|_| Vec::new(), ToOwned::to_owned);
    let value = output
        .value
        .into_data()
        .as_slice::<f32>()
        .ok()
        .and_then(|values| values.first().copied())
        .unwrap_or(0.0);

    (logits, value)
}

fn logits_to_probabilities(logits: &[f32]) -> Vec<f32> {
    let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut probabilities = logits
        .iter()
        .map(|logit| (logit - max_logit).exp())
        .collect::<Vec<_>>();
    let total = probabilities.iter().sum::<f32>();
    if total <= f32::EPSILON {
        return vec![1.0 / usize_to_f32(logits.len()); logits.len()];
    }
    for probability in &mut probabilities {
        *probability /= total;
    }
    probabilities
}

fn sample_from_probabilities(probabilities: &[f32], rng: &mut StdRng) -> usize {
    let target = rng.gen_range(0.0..1.0);
    let mut cumulative = 0.0;
    for (index, probability) in probabilities.iter().copied().enumerate() {
        cumulative += probability;
        if target <= cumulative {
            return index;
        }
    }
    probabilities.len().saturating_sub(1)
}

fn argmax_index(values: &[f32]) -> usize {
    values
        .iter()
        .copied()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map_or(0, |(index, _)| index)
}

fn action_from_index(index: usize) -> ControlAction {
    match index {
        0 => ControlAction::Noop,
        1 => ControlAction::Right,
        2 => ControlAction::RightA,
        3 => ControlAction::A,
        4 => ControlAction::RightB,
        _ => ControlAction::RightAB,
    }
}

fn action_index(action: ControlAction) -> usize {
    match action {
        ControlAction::Noop => 0,
        ControlAction::Right => 1,
        ControlAction::RightA => 2,
        ControlAction::A => 3,
        ControlAction::RightB => 4,
        ControlAction::RightAB => 5,
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
    if cfg.epochs == 0 {
        return Err(AiError::Unsupported("epochs must be greater than zero"));
    }
    if cfg.training_updates == 0 {
        return Err(AiError::Unsupported(
            "training_updates must be greater than zero",
        ));
    }
    if cfg.learning_rate <= 0.0 {
        return Err(AiError::Unsupported(
            "learning_rate must be greater than zero",
        ));
    }
    if !(0.0 < cfg.discount_gamma && cfg.discount_gamma <= 1.0) {
        return Err(AiError::Unsupported("discount_gamma must be within (0, 1]"));
    }
    if !(0.0 < cfg.gae_lambda && cfg.gae_lambda <= 1.0) {
        return Err(AiError::Unsupported("gae_lambda must be within (0, 1]"));
    }
    if cfg.clip_epsilon <= 0.0 {
        return Err(AiError::Unsupported(
            "clip_epsilon must be greater than zero",
        ));
    }
    if cfg.entropy_coefficient < 0.0 {
        return Err(AiError::Unsupported(
            "entropy_coefficient must be non-negative",
        ));
    }
    if cfg.value_loss_coefficient < 0.0 {
        return Err(AiError::Unsupported(
            "value_loss_coefficient must be non-negative",
        ));
    }
    if cfg.checkpoint_interval == 0 {
        return Err(AiError::Unsupported(
            "checkpoint_interval must be greater than zero",
        ));
    }

    Ok(())
}

fn mean_f32(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / usize_to_f32(values.len())
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn quantize_to_index(value: f32, max_index: usize) -> usize {
    value.max(0.0).min(max_index as f32).round() as usize
}

fn usize_to_f32(value: usize) -> f32 {
    let clamped = value.min(usize::from(u16::MAX));
    let clamped = u16::try_from(clamped).unwrap_or(u16::MAX);

    f32::from(clamped)
}

fn u32_to_f32(value: u32) -> f32 {
    let clamped = u16::try_from(value).unwrap_or(u16::MAX);

    f32::from(clamped)
}

struct SgdStep<'a, B: AutodiffBackend> {
    grads: &'a mut B::Gradients,
    learning_rate: f32,
}

impl<B: AutodiffBackend> ModuleMapper<B> for SgdStep<'_, B> {
    fn map_float<const D: usize>(&mut self, param: Param<Tensor<B, D>>) -> Param<Tensor<B, D>> {
        let Some(grad) = param.grad_remove(self.grads) else {
            return param;
        };
        let (id, tensor, mapper) = param.consume();
        let requires_grad = tensor.is_require_grad();
        let updated =
            Tensor::<B, D>::from_inner(tensor.inner() - grad.mul_scalar(self.learning_rate))
                .set_require_grad(requires_grad);

        Param::from_mapped_value(id, updated, mapper)
    }
}
