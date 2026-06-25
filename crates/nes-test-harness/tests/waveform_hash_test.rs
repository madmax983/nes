use nes_test_harness::{waveform_hash, audio_stats, rms_envelope, pearson_correlation, compare_waveforms};

#[test]
fn test_waveform_hash_coverage() {
    let empty: [i16; 0] = [];
    assert_ne!(waveform_hash(&empty), 0);
    assert_ne!(waveform_hash(&empty), 1);

    let a = [1i16, 2, 3];
    let b = [1i16, 2, 4];
    assert_ne!(waveform_hash(&a), waveform_hash(&b));
    assert_ne!(waveform_hash(&a), 0);
    assert_ne!(waveform_hash(&a), 1);
}

#[test]
fn test_audio_stats_coverage() {
    let empty: [i16; 0] = [];
    let stats = audio_stats(&empty);
    assert_eq!(stats.sample_count, 0);

    let positive = [10i16, 20i16];
    let p_stats = audio_stats(&positive);
    assert_eq!(p_stats.dc_offset, 15.0); // (10 + 20) / 2 = 15

    let clipping = [32767i16, 0i16];
    let c_stats = audio_stats(&clipping);
    assert_eq!(c_stats.clipping_ratio, 0.5);
}

#[test]
fn test_audio_stats_coverage_rms() {
    let samples = [10i16, 10i16];
    let stats = audio_stats(&samples);
    // RMS = sqrt((10*10 + 10*10) / 2) = sqrt(100) = 10
    // Mutated `value / value` -> sum_sq = 1 + 1 = 2 -> RMS = sqrt(2/2) = 1
    assert_eq!(stats.rms, 10.0);
}

#[test]
fn test_rms_envelope_coverage() {
    let empty: [i16; 0] = [];
    assert!(rms_envelope(&empty, 10).is_empty());

    let samples = [10i16];
    assert!(rms_envelope(&samples, 0).is_empty());

    let samples2 = [10i16, 10i16, 20i16];
    let env = rms_envelope(&samples2, 2);
    // Chunk 1: [10, 10] -> sum_sq = 200, n = 2 -> 100 -> sqrt = 10
    // Chunk 2: [20] -> sum_sq = 400, n = 1 -> 400 -> sqrt = 20
    assert_eq!(env.len(), 2);
    assert_eq!(env[0], 10.0);
    assert_eq!(env[1], 20.0);
}

#[test]
fn test_pearson_correlation_coverage() {
    let empty: [i16; 0] = [];
    assert_eq!(pearson_correlation(&empty, &empty), 0.0); // Covers `n < 2 -> 0.0`

    let single: [i16; 1] = [10];
    assert_eq!(pearson_correlation(&single, &single), 0.0); // Covers `n < 2 -> 0.0`

    let a = [1i16, 2, 3];
    let b = [3i16, 2, 1];
    assert_eq!(pearson_correlation(&a, &b), -1.0); // Negative correlation, catches replacement with 1.0
}

#[test]
fn test_pearson_correlation_coverage_denom() {
    let a = [1i16, 2, 3];
    let e = [1i16, 2, 2];
    let expected = pearson_correlation(&a, &e);
    assert!(expected > 0.85 && expected < 0.88); // 0.866
}

#[test]
fn test_pearson_correlation_n_less_than_or_equal_to_2() {
    let single: [i16; 1] = [10];
    let r = pearson_correlation(&single, &single);
    assert_eq!(r, 0.0);
}

#[test]
fn test_compare_waveforms_rms_ratio_coverage() {
    let empty: [i16; 0] = [];
    assert_eq!(compare_waveforms(&empty, &empty, 1024).rms_ratio, 0.0);

    // Test where rhs_stats.rms is 0
    let a = [100i16];
    let zeros = [0i16];
    assert_eq!(compare_waveforms(&a, &zeros, 1024).rms_ratio, 0.0);

    // Test where rhs_stats.rms > 0
    let ones = [1i16];
    assert_eq!(compare_waveforms(&a, &ones, 1024).rms_ratio, 100.0);
}
