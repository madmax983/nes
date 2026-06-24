import sys

def main():
    file_path = 'crates/nes-desktop/src/main.rs'
    with open(file_path, 'r') as f:
        content = f.read()

    old_code = '''    #[test]
    fn adaptive_delay_exact_targets_and_hysteresis_behave_as_expected() {'''

    new_code = '''    use proptest::prelude::*;
    proptest! {
        #[test]
        #[should_panic]
        fn havoc_test_recommended_input_delay_frames_panics_on_extreme_latency(
            rtt in any::<f64>(),
            jitter in any::<f64>(),
        ) {
            let _ = recommended_input_delay_frames(Some(rtt), jitter, 0, 10, 5);
        }
    }

    #[test]
    fn adaptive_delay_exact_targets_and_hysteresis_behave_as_expected() {'''

    if old_code in content:
        content = content.replace(old_code, new_code)
        with open(file_path, 'w') as f:
            f.write(content)
        print('success')
    else:
        print('could not find old code block')

if __name__ == '__main__':
    main()
