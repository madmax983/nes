use std::collections::HashSet;
use std::fs;

use nes_core::{AUDIO_CHUNK_SAMPLES, Button, Command, NesCore};
use nes_test_harness::smb_rom_path;

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_rom_loads_and_exposes_reset_vector() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    let info = core.load_ines_rom(&bytes).expect("failed to load SMB ROM");
    assert!(
        info.reset_pc >= 0x8000,
        "unexpected reset vector {:04X}",
        info.reset_pc
    );
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_boot_window_runs_without_unknown_opcode() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    for step in 0..20_000_u32 {
        core.execute(Command::StepCpu).unwrap_or_else(|err| {
            panic!(
                "SMB boot window failed at step {step}, pc={:04X}: {err}",
                core.cpu_pc()
            )
        });
    }
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_boot_frames_use_nes_palette_bounds_and_stable_delta() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    for _ in 0..180 {
        core.execute(Command::StepFrame)
            .unwrap_or_else(|err| panic!("SMB frame step failed, pc={:04X}: {err}", core.cpu_pc()));
    }

    let frame_a = core.framebuffer_rgba();
    core.execute(Command::StepFrame).unwrap_or_else(|err| {
        panic!(
            "SMB next frame step failed, pc={:04X}: {err}",
            core.cpu_pc()
        )
    });
    let frame_b = core.framebuffer_rgba();

    let unique_colors = rgb_unique_count(&frame_a);
    assert!(
        (4..=64).contains(&unique_colors),
        "expected NES palette-bounded colors, got {unique_colors} unique colors"
    );

    let changed = changed_pixel_count(&frame_a, &frame_b);
    let total = frame_a.len() / 4;
    assert!(
        changed * 100 < total * 25,
        "too many pixels changed between adjacent boot frames: {changed}/{total}"
    );
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_start_input_changes_execution_trajectory() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");

    let mut baseline = NesCore::new();
    baseline
        .load_ines_rom(&bytes)
        .expect("failed to load SMB ROM");
    let baseline_reads = collect_controller_reads(&mut baseline, 400_000);

    let mut with_start = NesCore::new();
    with_start
        .load_ines_rom(&bytes)
        .expect("failed to load SMB ROM");
    with_start
        .execute(Command::PressButton(Button::Start))
        .unwrap();
    let with_start_reads = collect_controller_reads(&mut with_start, 400_000);

    assert!(
        !baseline_reads.is_empty(),
        "expected SMB to read controller port during boot/title loop"
    );
    assert_eq!(
        baseline_reads.len(),
        with_start_reads.len(),
        "controller polling cadence should stay deterministic across inputs"
    );
    assert_ne!(
        baseline_reads, with_start_reads,
        "holding Start should alter observed controller port read stream"
    );
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_start_and_move_right_keeps_video_progressing() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    for _ in 0..180 {
        core.execute(Command::StepFrame).unwrap();
    }

    core.execute(Command::PressButton(Button::Start)).unwrap();
    for _ in 0..4 {
        core.execute(Command::StepFrame).unwrap();
    }
    core.execute(Command::ReleaseButton(Button::Start)).unwrap();

    core.execute(Command::PressButton(Button::Right)).unwrap();
    let mut signatures = HashSet::new();
    for _ in 0..600 {
        core.execute(Command::StepFrame).unwrap();
        let frame = core.framebuffer_rgba();
        signatures.insert(frame_signature(&frame));
    }
    core.execute(Command::ReleaseButton(Button::Right)).unwrap();

    assert!(
        signatures.len() >= 32,
        "expected ongoing visual progression after entering gameplay; got {} unique frame signatures",
        signatures.len()
    );
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_does_not_stall_in_sprite_zero_wait_loop() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    let mut current_stall_frames = 0_u32;
    let mut max_stall_frames = 0_u32;
    for _ in 0..1_500_u32 {
        core.execute(Command::StepFrame).unwrap();
        if core.cpu_pc() == 0x8155 {
            current_stall_frames = current_stall_frames.saturating_add(1);
            max_stall_frames = max_stall_frames.max(current_stall_frames);
        } else {
            current_stall_frames = 0;
        }
    }

    assert!(
        max_stall_frames < 120,
        "SMB stalled in sprite-0 wait loop for {max_stall_frames} consecutive frames at PC $8155"
    );
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_run_and_jump_keeps_progressing() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    for _ in 0..180 {
        core.execute(Command::StepFrame).unwrap();
    }

    core.execute(Command::PressButton(Button::Start)).unwrap();
    for _ in 0..4 {
        core.execute(Command::StepFrame).unwrap();
    }
    core.execute(Command::ReleaseButton(Button::Start)).unwrap();

    core.execute(Command::PressButton(Button::Right)).unwrap();

    let mut signatures = HashSet::new();
    let mut last_pc = 0_u16;
    let mut current_pc_stall = 0_u32;
    let mut max_pc_stall = 0_u32;
    for frame in 0..1_200_u32 {
        if frame % 45 == 0 {
            core.execute(Command::PressButton(Button::A)).unwrap();
        }
        if frame % 45 == 8 {
            core.execute(Command::ReleaseButton(Button::A)).unwrap();
        }

        core.execute(Command::StepFrame).unwrap();
        signatures.insert(frame_signature(&core.framebuffer_rgba()));

        let pc = core.cpu_pc();
        if pc == last_pc {
            current_pc_stall = current_pc_stall.saturating_add(1);
            max_pc_stall = max_pc_stall.max(current_pc_stall);
        } else {
            current_pc_stall = 0;
            last_pc = pc;
        }
    }

    core.execute(Command::ReleaseButton(Button::Right)).unwrap();
    core.execute(Command::ReleaseButton(Button::A)).unwrap();

    assert!(
        signatures.len() >= 80,
        "expected substantial frame diversity while running+jumping, got {} unique signatures",
        signatures.len()
    );
    assert!(
        max_pc_stall < 180,
        "cpu appeared stalled too long at one pc: {max_pc_stall} consecutive sampled frames"
    );
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_run_and_jump_audio_chunks_stay_live() {
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    for _ in 0..180 {
        core.execute(Command::StepFrame).unwrap();
    }

    core.execute(Command::PressButton(Button::Start)).unwrap();
    for _ in 0..4 {
        core.execute(Command::StepFrame).unwrap();
    }
    core.execute(Command::ReleaseButton(Button::Start)).unwrap();

    core.execute(Command::PressButton(Button::Right)).unwrap();

    let mut signatures = HashSet::new();
    let mut last_pc = 0_u16;
    let mut current_pc_stall = 0_u32;
    let mut max_pc_stall = 0_u32;
    let mut non_silent_chunks = 0_u32;
    for frame in 0..1_200_u32 {
        if frame % 45 == 0 {
            core.execute(Command::PressButton(Button::A)).unwrap();
        }
        if frame % 45 == 8 {
            core.execute(Command::ReleaseButton(Button::A)).unwrap();
        }

        core.execute(Command::StepFrame).unwrap();
        signatures.insert(frame_signature(&core.framebuffer_rgba()));

        let chunk = core.audio_chunk_i16();
        assert_eq!(
            chunk.len(),
            AUDIO_CHUNK_SAMPLES,
            "audio chunk length drifted from expected fixed cadence"
        );
        if chunk.iter().any(|sample| *sample != 0) {
            non_silent_chunks = non_silent_chunks.saturating_add(1);
        }

        let pc = core.cpu_pc();
        if pc == last_pc {
            current_pc_stall = current_pc_stall.saturating_add(1);
            max_pc_stall = max_pc_stall.max(current_pc_stall);
        } else {
            current_pc_stall = 0;
            last_pc = pc;
        }
    }

    core.execute(Command::ReleaseButton(Button::Right)).unwrap();
    core.execute(Command::ReleaseButton(Button::A)).unwrap();

    assert!(
        signatures.len() >= 80,
        "expected substantial frame diversity while running+jumping, got {} unique signatures",
        signatures.len()
    );
    assert!(
        max_pc_stall < 180,
        "cpu appeared stalled too long at one pc: {max_pc_stall} consecutive sampled frames"
    );
    assert!(
        non_silent_chunks >= 200,
        "audio output was unexpectedly silent during active gameplay ({non_silent_chunks}/1200 non-silent chunks)"
    );
}

#[test]
#[ignore = "requires roms.smb or desktop.rom_path in nes.toml"]
fn smb_run_and_jump_keeps_progressing_under_cpu_budget_loop() {
    const CPU_STEPS_PER_HOST_FRAME: u32 = 10_000;
    let rom_path = smb_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    for _ in 0..180 {
        for _ in 0..CPU_STEPS_PER_HOST_FRAME {
            core.execute(Command::StepCpu).unwrap();
        }
    }

    core.execute(Command::PressButton(Button::Start)).unwrap();
    for _ in 0..4 {
        for _ in 0..CPU_STEPS_PER_HOST_FRAME {
            core.execute(Command::StepCpu).unwrap();
        }
    }
    core.execute(Command::ReleaseButton(Button::Start)).unwrap();
    core.execute(Command::PressButton(Button::Right)).unwrap();

    let mut signatures = HashSet::new();
    let mut last_pc = 0_u16;
    let mut current_pc_stall = 0_u32;
    let mut max_pc_stall = 0_u32;
    for frame in 0..1_200_u32 {
        if frame % 45 == 0 {
            core.execute(Command::PressButton(Button::A)).unwrap();
        }
        if frame % 45 == 8 {
            core.execute(Command::ReleaseButton(Button::A)).unwrap();
        }

        for _ in 0..CPU_STEPS_PER_HOST_FRAME {
            core.execute(Command::StepCpu).unwrap();
        }
        signatures.insert(frame_signature(&core.framebuffer_rgba()));

        let pc = core.cpu_pc();
        if pc == last_pc {
            current_pc_stall = current_pc_stall.saturating_add(1);
            max_pc_stall = max_pc_stall.max(current_pc_stall);
        } else {
            current_pc_stall = 0;
            last_pc = pc;
        }
    }

    core.execute(Command::ReleaseButton(Button::Right)).unwrap();
    core.execute(Command::ReleaseButton(Button::A)).unwrap();

    assert!(
        signatures.len() >= 80,
        "expected substantial frame diversity under cpu-budget loop, got {} unique signatures",
        signatures.len()
    );
    assert!(
        max_pc_stall < 180,
        "cpu appeared stalled too long at one pc under cpu-budget loop: {max_pc_stall} consecutive sampled frames"
    );
}

fn collect_controller_reads(core: &mut NesCore, steps: u32) -> Vec<u8> {
    let mut reads = Vec::new();
    for _ in 0..steps {
        core.execute(Command::StepCpu).unwrap();
        for access in core.last_cpu_bus_trace() {
            if access.addr == 0x4016 && access.kind == nes_core::cpu::CpuBusAccessKind::Read {
                reads.push(access.value);
            }
        }
    }
    reads
}

fn rgb_unique_count(frame: &[u8]) -> usize {
    let mut colors = HashSet::new();
    for px in frame.as_chunks::<4>().0 {
        colors.insert((px[0], px[1], px[2]));
    }
    colors.len()
}

fn changed_pixel_count(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(lhs, rhs)| lhs[0] != rhs[0] || lhs[1] != rhs[1] || lhs[2] != rhs[2])
        .count()
}

fn frame_signature(frame: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for chunk in frame.as_chunks::<64>().0 {
        hash ^= u64::from(chunk[0]);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash
}
