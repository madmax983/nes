use burn_autodiff::Autodiff;
use burn_ndarray::NdArray;
use burn_tensor::{Tensor, backend::Backend};
use nes_ai::{
    actions::ControlAction,
    model::{HybridObservationBatch, HybridPolicyValueConfig},
};

type TestBackend = Autodiff<NdArray<f32>>;

#[test]
fn policy_value_model_emits_expected_output_shapes() {
    let device = <TestBackend as Backend>::Device::default();
    let model = HybridPolicyValueConfig::new(4, 8, ControlAction::action_count())
        .init::<TestBackend>(&device);
    let batch = HybridObservationBatch {
        frames: Tensor::<TestBackend, 4>::zeros([3, 4, 84, 84], &device),
        features: Tensor::<TestBackend, 2>::zeros([3, 8], &device),
    };

    let output = model.forward(batch);

    assert_eq!(
        output.policy_logits.dims(),
        [3, ControlAction::action_count()]
    );
    assert_eq!(output.value.dims(), [3, 1]);
}

#[test]
fn policy_value_model_supports_nondefault_observation_sizes() {
    let device = <TestBackend as Backend>::Device::default();
    let model = HybridPolicyValueConfig::new(4, 8, ControlAction::action_count())
        .with_observation_width(64)
        .with_observation_height(48)
        .init::<TestBackend>(&device);
    let batch = HybridObservationBatch {
        frames: Tensor::<TestBackend, 4>::zeros([2, 4, 48, 64], &device),
        features: Tensor::<TestBackend, 2>::zeros([2, 8], &device),
    };

    let output = model.forward(batch);

    assert_eq!(
        output.policy_logits.dims(),
        [2, ControlAction::action_count()]
    );
    assert_eq!(output.value.dims(), [2, 1]);
}

#[test]
#[should_panic(expected = "frame_stack must be greater than zero")]
fn policy_value_model_rejects_zero_frame_stack() {
    let device = <TestBackend as Backend>::Device::default();
    let _model = HybridPolicyValueConfig::new(0, 8, ControlAction::action_count())
        .init::<TestBackend>(&device);
}

#[test]
#[should_panic(expected = "observation width must be at least 20 pixels")]
fn policy_value_model_rejects_too_small_observation_width() {
    let device = <TestBackend as Backend>::Device::default();
    let _model = HybridPolicyValueConfig::new(4, 8, ControlAction::action_count())
        .with_observation_width(19)
        .init::<TestBackend>(&device);
}
