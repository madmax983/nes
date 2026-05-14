//! Audio Processing Unit (APU) model and sample generation.
//!
//! Implements pulse/triangle/noise/DMC channels, frame sequencer timing,
//! IRQ behavior, and mixed PCM sample generation for host playback.

use std::collections::VecDeque;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::constants::AUDIO_SAMPLE_RATE;

const CPU_CLOCK_HZ: u64 = 1_789_773;
const MAX_SAMPLE_AMPLITUDE: f32 = 11_500.0;
const MAX_QUEUED_SAMPLES: usize = AUDIO_SAMPLE_RATE as usize;
const HPF_90_R_Q16: i64 = 64_701;
const HPF_440_R_Q16: i64 = 61_554;
const LPF_14K_A_Q16: i64 = 56_619;
const SOFT_CLIP_KNEE: i64 = 28_000;
const SOFT_CLIP_RATIO_SHIFT: u32 = 2;

const FRAME_STEP_1: u16 = 3_729;
const FRAME_STEP_2: u16 = 7_457;
const FRAME_STEP_3: u16 = 11_186;
const FRAME_STEP_4: u16 = 14_916;
const FRAME_STEP_5: u16 = 18_641;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1], // 25% negated
];

const TRIANGLE_TABLE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1_016, 2_034, 4_068,
];

const DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 85, 72, 54,
];

const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// DMC DMA fetch request produced by the APU.
pub struct DmcDmaRequest {
    /// CPU address to fetch sample byte from.
    pub addr: u16,
    /// CPU cycles to stall while performing the fetch.
    pub stall_cycles: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PulseChannel {
    is_pulse1: bool,
    enabled: bool,
    control: u8,
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_divider: u8,
    sweep_reload: bool,
    timer_reload: u16,
    timer_counter: u16,
    duty_step: u8,
    length_counter: u8,
    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
}

impl PulseChannel {
    fn new(is_pulse1: bool) -> Self {
        Self {
            is_pulse1,
            enabled: false,
            control: 0x30,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_divider: 0,
            sweep_reload: false,
            timer_reload: 0,
            timer_counter: 0,
            duty_step: 0,
            length_counter: 0,
            envelope_start: true,
            envelope_divider: 0,
            envelope_decay: 15,
        }
    }

    fn boot_tone() -> Self {
        let mut channel = Self::new(true);
        channel.enabled = true;
        channel.control = 0x9F; // duty=2, constant volume=15
        channel.timer_reload = 0x80;
        channel.timer_counter = channel.timer_reload;
        channel.length_counter = LENGTH_TABLE[10];
        channel.envelope_decay = 15;
        channel
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn write_control(&mut self, value: u8) {
        self.control = value;
    }

    fn write_sweep(&mut self, value: u8) {
        self.sweep_enabled = value & 0x80 != 0;
        self.sweep_period = (value >> 4) & 0x07;
        self.sweep_negate = value & 0x08 != 0;
        self.sweep_shift = value & 0x07;
        self.sweep_reload = true;
    }

    fn write_timer_low(&mut self, value: u8) {
        self.timer_reload = (self.timer_reload & 0x0700) | u16::from(value);
    }

    fn write_timer_high(&mut self, value: u8) {
        self.timer_reload = (self.timer_reload & 0x00FF) | (u16::from(value & 0x07) << 8);
        if self.enabled {
            self.length_counter = LENGTH_TABLE[((value >> 3) & 0x1F) as usize];
        }
        self.duty_step = 0;
        self.envelope_start = true;
    }

    fn clock_timer(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_reload;
            self.duty_step = (self.duty_step + 1) & 0x07;
        } else {
            self.timer_counter = self.timer_counter.saturating_sub(1);
        }
    }

    fn clock_quarter_frame(&mut self) {
        if self.envelope_start {
            self.envelope_start = false;
            self.envelope_decay = 15;
            self.envelope_divider = self.envelope_period();
            return;
        }

        if self.envelope_divider == 0 {
            self.envelope_divider = self.envelope_period();
            if self.envelope_decay == 0 {
                if self.length_halt() {
                    self.envelope_decay = 15;
                }
            } else {
                self.envelope_decay = self.envelope_decay.saturating_sub(1);
            }
            return;
        }

        self.envelope_divider = self.envelope_divider.saturating_sub(1);
    }

    fn clock_half_frame(&mut self) {
        if !self.length_halt() && self.length_counter > 0 {
            self.length_counter = self.length_counter.saturating_sub(1);
        }
        self.clock_sweep();
    }

    fn output(&self) -> u8 {
        if self.length_counter == 0 || self.sweep_muted() {
            return 0;
        }
        let duty = usize::from((self.control >> 6) & 0x03);
        let step = usize::from(self.duty_step);
        if DUTY_TABLE[duty][step] == 0 {
            0
        } else {
            self.volume()
        }
    }

    fn length_counter(&self) -> u8 {
        self.length_counter
    }

    fn length_halt(&self) -> bool {
        self.control & 0x20 != 0
    }

    fn envelope_period(&self) -> u8 {
        self.control & 0x0F
    }

    fn volume(&self) -> u8 {
        if self.control & 0x10 != 0 {
            self.control & 0x0F
        } else {
            self.envelope_decay
        }
    }

    fn clock_sweep(&mut self) {
        let target = self.sweep_target_period();
        let mute = self.sweep_mute_for_target(target);

        if self.sweep_divider == 0 {
            if self.sweep_enabled && self.sweep_shift > 0 && !mute {
                self.timer_reload = target as u16;
            }
            self.sweep_divider = self.sweep_period;
        } else {
            self.sweep_divider = self.sweep_divider.saturating_sub(1);
        }

        if self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        }
    }

    fn sweep_target_period(&self) -> i32 {
        let period = i32::from(self.timer_reload);
        let delta = period >> self.sweep_shift;
        if self.sweep_negate {
            period - delta - if self.is_pulse1 { 1 } else { 0 }
        } else {
            period + delta
        }
    }

    fn sweep_muted(&self) -> bool {
        self.sweep_mute_for_target(self.sweep_target_period())
    }

    fn sweep_mute_for_target(&self, target: i32) -> bool {
        self.timer_reload < 8 || !(0..=0x7FF).contains(&target)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct TriangleChannel {
    enabled: bool,
    control: u8,
    linear_reload_value: u8,
    linear_counter: u8,
    linear_reload_flag: bool,
    timer_reload: u16,
    timer_counter: u16,
    sequence_step: u8,
    length_counter: u8,
}

impl TriangleChannel {
    fn new() -> Self {
        Self {
            enabled: false,
            control: 0x80,
            linear_reload_value: 0,
            linear_counter: 0,
            linear_reload_flag: false,
            timer_reload: 0,
            timer_counter: 0,
            sequence_step: 0,
            length_counter: 0,
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn write_linear_control(&mut self, value: u8) {
        self.control = value;
        self.linear_reload_value = value & 0x7F;
    }

    fn write_timer_low(&mut self, value: u8) {
        self.timer_reload = (self.timer_reload & 0x0700) | u16::from(value);
    }

    fn write_timer_high(&mut self, value: u8) {
        self.timer_reload = (self.timer_reload & 0x00FF) | (u16::from(value & 0x07) << 8);
        if self.enabled {
            self.length_counter = LENGTH_TABLE[((value >> 3) & 0x1F) as usize];
        }
        self.linear_reload_flag = true;
    }

    fn clock_timer(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_reload;
            if self.length_counter > 0 && self.linear_counter > 0 && self.timer_reload >= 2 {
                self.sequence_step = (self.sequence_step + 1) & 0x1F;
            }
        } else {
            self.timer_counter = self.timer_counter.saturating_sub(1);
        }
    }

    fn clock_quarter_frame(&mut self) {
        if self.linear_reload_flag {
            self.linear_counter = self.linear_reload_value;
        } else if self.linear_counter > 0 {
            self.linear_counter = self.linear_counter.saturating_sub(1);
        }

        if !self.length_halt() {
            self.linear_reload_flag = false;
        }
    }

    fn clock_half_frame(&mut self) {
        if !self.length_halt() && self.length_counter > 0 {
            self.length_counter = self.length_counter.saturating_sub(1);
        }
    }

    fn output(&self) -> u8 {
        if self.length_counter == 0 || self.linear_counter == 0 || self.timer_reload < 2 {
            0
        } else {
            TRIANGLE_TABLE[usize::from(self.sequence_step)]
        }
    }

    fn length_counter(&self) -> u8 {
        self.length_counter
    }

    fn length_halt(&self) -> bool {
        self.control & 0x80 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct NoiseChannel {
    enabled: bool,
    control: u8,
    mode: bool,
    period_index: u8,
    timer_reload: u16,
    timer_counter: u16,
    length_counter: u8,
    shift_register: u16,
    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
}

impl NoiseChannel {
    fn new() -> Self {
        Self {
            enabled: false,
            control: 0x30,
            mode: false,
            period_index: 0,
            timer_reload: NOISE_PERIOD_TABLE[0],
            timer_counter: NOISE_PERIOD_TABLE[0],
            length_counter: 0,
            shift_register: 1,
            envelope_start: true,
            envelope_divider: 0,
            envelope_decay: 15,
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn write_control(&mut self, value: u8) {
        self.control = value;
    }

    fn write_period(&mut self, value: u8) {
        self.mode = value & 0x80 != 0;
        self.period_index = value & 0x0F;
        self.timer_reload = NOISE_PERIOD_TABLE[usize::from(self.period_index)];
    }

    fn write_length(&mut self, value: u8) {
        if self.enabled {
            self.length_counter = LENGTH_TABLE[((value >> 3) & 0x1F) as usize];
        }
        self.envelope_start = true;
    }

    fn clock_timer(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_reload;
            let tap = if self.mode { 6 } else { 1 };
            let feedback = (self.shift_register & 0x0001) ^ ((self.shift_register >> tap) & 0x0001);
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        } else {
            self.timer_counter = self.timer_counter.saturating_sub(1);
        }
    }

    fn clock_quarter_frame(&mut self) {
        if self.envelope_start {
            self.envelope_start = false;
            self.envelope_decay = 15;
            self.envelope_divider = self.envelope_period();
            return;
        }

        if self.envelope_divider == 0 {
            self.envelope_divider = self.envelope_period();
            if self.envelope_decay == 0 {
                if self.length_halt() {
                    self.envelope_decay = 15;
                }
            } else {
                self.envelope_decay = self.envelope_decay.saturating_sub(1);
            }
            return;
        }

        self.envelope_divider = self.envelope_divider.saturating_sub(1);
    }

    fn clock_half_frame(&mut self) {
        if !self.length_halt() && self.length_counter > 0 {
            self.length_counter = self.length_counter.saturating_sub(1);
        }
    }

    fn output(&self) -> u8 {
        if self.length_counter == 0 || self.shift_register & 0x0001 != 0 {
            0
        } else {
            self.volume()
        }
    }

    fn length_counter(&self) -> u8 {
        self.length_counter
    }

    fn length_halt(&self) -> bool {
        self.control & 0x20 != 0
    }

    fn envelope_period(&self) -> u8 {
        self.control & 0x0F
    }

    fn volume(&self) -> u8 {
        if self.control & 0x10 != 0 {
            self.control & 0x0F
        } else {
            self.envelope_decay
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct DmcChannel {
    enabled: bool,
    irq_enabled: bool,
    loop_flag: bool,
    rate_index: u8,
    output_level: u8,
    sample_addr_reg: u8,
    sample_len_reg: u8,
    current_addr: u16,
    bytes_remaining: u16,
    sample_buffer: Option<u8>,
    shift_register: u8,
    bits_remaining: u8,
    silence: bool,
    timer_counter: u16,
    dma_pending: bool,
    irq_pending: bool,
    fetch_count: u64,
}

impl DmcChannel {
    fn new() -> Self {
        Self {
            enabled: false,
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            output_level: 0,
            sample_addr_reg: 0,
            sample_len_reg: 0,
            current_addr: 0xC000,
            bytes_remaining: 0,
            sample_buffer: None,
            shift_register: 0,
            bits_remaining: 8,
            silence: true,
            timer_counter: DMC_RATE_TABLE[0].saturating_sub(1),
            dma_pending: false,
            irq_pending: false,
            fetch_count: 0,
        }
    }

    fn write_control(&mut self, value: u8) {
        self.irq_enabled = value & 0x80 != 0;
        self.loop_flag = value & 0x40 != 0;
        self.rate_index = value & 0x0F;
        if !self.irq_enabled {
            self.irq_pending = false;
        }
    }

    fn write_direct_load(&mut self, value: u8) {
        self.output_level = value & 0x7F;
    }

    fn write_sample_addr(&mut self, value: u8) {
        self.sample_addr_reg = value;
    }

    fn write_sample_len(&mut self, value: u8) {
        self.sample_len_reg = value;
    }

    fn write_status_enable(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.irq_pending = false;
        if enabled {
            if self.bytes_remaining == 0 {
                self.restart_sample();
            }
        } else {
            self.dma_pending = false;
            self.bytes_remaining = 0;
        }
    }

    fn load_sample(&mut self, sample: u8) {
        self.sample_buffer = Some(sample);
        self.complete_pending_fetch();
    }

    fn step_timer(&mut self) -> Option<DmcDmaRequest> {
        if self.timer_counter == 0 {
            self.timer_counter = self.rate_period_counter();
            self.clock_output();
        } else {
            self.timer_counter = self.timer_counter.saturating_sub(1);
        }

        if !self.dma_pending && self.sample_buffer.is_none() && self.bytes_remaining > 0 {
            self.dma_pending = true;
            let request = DmcDmaRequest {
                addr: self.current_addr,
                stall_cycles: 4,
            };
            self.fetch_count = self.fetch_count.saturating_add(1);
            return Some(request);
        }

        None
    }

    fn clock_output(&mut self) {
        if !self.silence {
            if self.shift_register & 1 != 0 {
                if self.output_level <= 125 {
                    self.output_level = self.output_level.saturating_add(2);
                }
            } else if self.output_level >= 2 {
                self.output_level = self.output_level.saturating_sub(2);
            }
        }

        self.shift_register >>= 1;
        self.bits_remaining = self.bits_remaining.saturating_sub(1);
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;
            if let Some(sample) = self.sample_buffer.take() {
                self.shift_register = sample;
                self.silence = false;
            } else {
                self.silence = true;
            }
        }
    }

    fn restart_sample(&mut self) {
        self.current_addr = 0xC000 | (u16::from(self.sample_addr_reg) << 6);
        self.bytes_remaining = (u16::from(self.sample_len_reg) << 4) | 1;
    }

    fn complete_pending_fetch(&mut self) {
        if !self.dma_pending {
            return;
        }
        self.dma_pending = false;
        self.current_addr = if self.current_addr == 0xFFFF {
            0x8000
        } else {
            self.current_addr.wrapping_add(1)
        };
        self.bytes_remaining = self.bytes_remaining.saturating_sub(1);
        if self.bytes_remaining == 0 {
            if self.loop_flag {
                self.restart_sample();
            } else if self.irq_enabled {
                self.irq_pending = true;
            }
        }
    }

    fn rate_period(&self) -> u16 {
        DMC_RATE_TABLE[usize::from(self.rate_index)]
    }

    fn rate_period_counter(&self) -> u16 {
        self.rate_period().saturating_sub(1)
    }

    fn output(&self) -> u8 {
        self.output_level
    }

    fn active(&self) -> bool {
        self.bytes_remaining > 0
    }

    fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    fn clear_irq(&mut self) {
        self.irq_pending = false;
    }

    fn bytes_remaining(&self) -> u16 {
        self.bytes_remaining
    }

    fn fetch_count(&self) -> u64 {
        self.fetch_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Serializable APU snapshot.
///
/// **Performance optimization:** The `samples` field uses `VecDeque<i16>` instead of `Vec<i16>`
/// to match `Apu`'s internal storage exactly. This avoids an expensive `Vec::from()` allocation
/// and underlying buffer rotation (to make the ring buffer contiguous) on every snapshot.
/// The APU channel structs (`PulseChannel`, `TriangleChannel`, etc.) derive `Copy` rather than
/// relying on `.clone()` since they contain only scalar primitives, replacing deep clones with
/// fast memcpys.
pub struct ApuSnapshot {
    cpu_cycles: u64,
    frame_cycle: u16,
    quarter_frame_ticks: u64,
    half_frame_ticks: u64,
    frame_irq_pending: bool,
    frame_irq_inhibit: bool,
    frame_mode_5: bool,
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,
    sample_accumulator: u64,
    hp90_prev_out_q16: i64,
    hp90_prev_in_q16: i64,
    hp440_prev_out_q16: i64,
    hp440_prev_in_q16: i64,
    lp14k_prev_out_q16: i64,
    samples: std::collections::VecDeque<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Runtime APU state and channel graph.
pub struct Apu {
    cpu_cycles: u64,
    frame_cycle: u16,
    quarter_frame_ticks: u64,
    half_frame_ticks: u64,
    frame_irq_pending: bool,
    frame_irq_inhibit: bool,
    frame_mode_5: bool,
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,
    sample_accumulator: u64,
    hp90_prev_out_q16: i64,
    hp90_prev_in_q16: i64,
    hp440_prev_out_q16: i64,
    hp440_prev_in_q16: i64,
    lp14k_prev_out_q16: i64,
    samples: VecDeque<i16>,
}

impl Apu {
    /// Creates a power-on initialized APU.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu_cycles: 0,
            frame_cycle: 0,
            quarter_frame_ticks: 0,
            half_frame_ticks: 0,
            frame_irq_pending: false,
            frame_irq_inhibit: false,
            frame_mode_5: false,
            pulse1: PulseChannel::boot_tone(),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            sample_accumulator: 0,
            hp90_prev_out_q16: 0,
            hp90_prev_in_q16: 0,
            hp440_prev_out_q16: 0,
            hp440_prev_in_q16: 0,
            lp14k_prev_out_q16: 0,
            samples: VecDeque::with_capacity(MAX_QUEUED_SAMPLES),
        }
    }

    /// Resets the APU to power-on state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Captures full APU snapshot.
    #[must_use]
    pub fn snapshot(&self) -> ApuSnapshot {
        ApuSnapshot {
            cpu_cycles: self.cpu_cycles,
            frame_cycle: self.frame_cycle,
            quarter_frame_ticks: self.quarter_frame_ticks,
            half_frame_ticks: self.half_frame_ticks,
            frame_irq_pending: self.frame_irq_pending,
            frame_irq_inhibit: self.frame_irq_inhibit,
            frame_mode_5: self.frame_mode_5,
            pulse1: self.pulse1,
            pulse2: self.pulse2,
            triangle: self.triangle,
            noise: self.noise,
            dmc: self.dmc,
            sample_accumulator: self.sample_accumulator,
            hp90_prev_out_q16: self.hp90_prev_out_q16,
            hp90_prev_in_q16: self.hp90_prev_in_q16,
            hp440_prev_out_q16: self.hp440_prev_out_q16,
            hp440_prev_in_q16: self.hp440_prev_in_q16,
            lp14k_prev_out_q16: self.lp14k_prev_out_q16,
            samples: self.samples.clone(),
        }
    }

    /// Restores APU from snapshot.
    pub fn restore(&mut self, snapshot: ApuSnapshot) {
        self.cpu_cycles = snapshot.cpu_cycles;
        self.frame_cycle = snapshot.frame_cycle;
        self.quarter_frame_ticks = snapshot.quarter_frame_ticks;
        self.half_frame_ticks = snapshot.half_frame_ticks;
        self.frame_irq_pending = snapshot.frame_irq_pending;
        self.frame_irq_inhibit = snapshot.frame_irq_inhibit;
        self.frame_mode_5 = snapshot.frame_mode_5;
        self.pulse1 = snapshot.pulse1;
        self.pulse2 = snapshot.pulse2;
        self.triangle = snapshot.triangle;
        self.noise = snapshot.noise;
        self.dmc = snapshot.dmc;
        self.sample_accumulator = snapshot.sample_accumulator;
        self.hp90_prev_out_q16 = snapshot.hp90_prev_out_q16;
        self.hp90_prev_in_q16 = snapshot.hp90_prev_in_q16;
        self.hp440_prev_out_q16 = snapshot.hp440_prev_out_q16;
        self.hp440_prev_in_q16 = snapshot.hp440_prev_in_q16;
        self.lp14k_prev_out_q16 = snapshot.lp14k_prev_out_q16;
        self.samples = snapshot.samples;
    }

    /// Writes an APU/MMIO register (`$4000-$4017`).
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000 => self.pulse1.write_control(value),
            0x4001 => self.pulse1.write_sweep(value),
            0x4002 => self.pulse1.write_timer_low(value),
            0x4003 => self.pulse1.write_timer_high(value),
            0x4004 => self.pulse2.write_control(value),
            0x4005 => self.pulse2.write_sweep(value),
            0x4006 => self.pulse2.write_timer_low(value),
            0x4007 => self.pulse2.write_timer_high(value),
            0x4008 => self.triangle.write_linear_control(value),
            0x400A => self.triangle.write_timer_low(value),
            0x400B => self.triangle.write_timer_high(value),
            0x400C => self.noise.write_control(value),
            0x400E => self.noise.write_period(value),
            0x400F => self.noise.write_length(value),
            0x4010 => self.dmc.write_control(value),
            0x4011 => self.dmc.write_direct_load(value),
            0x4012 => self.dmc.write_sample_addr(value),
            0x4013 => self.dmc.write_sample_len(value),
            0x4015 => self.write_status(value),
            0x4017 => self.write_frame_counter(value),
            _ => {}
        }
    }

    /// Steps one CPU cycle worth of APU timing.
    ///
    /// **Why this optimization matters:**
    /// We defer the expensive per-sample computations (like `raw_mixed_sample`)
    /// inside the rate-limiting conditional branch (`sample_accumulator >= CPU_CLOCK_HZ`).
    /// This prevents them from executing pointlessly on every single CPU cycle,
    /// significantly improving the `step_cpu_cycle` throughput.
    ///
    /// Returns an optional DMC DMA request that the host/core must service.
    pub fn step_cpu_cycle(&mut self, paused: bool) -> Option<DmcDmaRequest> {
        self.cpu_cycles = self.cpu_cycles.saturating_add(1);
        self.frame_cycle = self.frame_cycle.saturating_add(1);

        self.clock_frame_sequencer();
        let dmc_request = self.clock_timers();

        self.sample_accumulator = self
            .sample_accumulator
            .saturating_add(u64::from(AUDIO_SAMPLE_RATE));
        while self.sample_accumulator >= CPU_CLOCK_HZ {
            self.sample_accumulator -= CPU_CLOCK_HZ;
            let raw_sample = self.raw_mixed_sample(paused);
            let filtered_sample = self.apply_output_filters(raw_sample);
            self.samples.push_back(filtered_sample);
            if self.samples.len() > MAX_QUEUED_SAMPLES {
                let _ = self.samples.pop_front();
            }
        }

        dmc_request
    }

    /// Loads one fetched DMC sample byte.
    pub fn load_dmc_sample(&mut self, sample: u8) {
        self.dmc.load_sample(sample);
    }

    /// Fills exactly `buffer.len()` PCM samples into the provided buffer.
    ///
    /// If insufficient samples are queued, the APU will keep stepping until
    /// enough samples are generated.
    pub fn fill_samples(&mut self, buffer: &mut [i16], paused: bool) {
        let mut idx = 0;
        let count = buffer.len();
        while idx < count {
            if let Some(sample) = self.samples.pop_front() {
                buffer[idx] = sample;
                idx += 1;
                continue;
            }
            let _ = self.step_cpu_cycle(paused);
        }
    }

    /// Reads status register with side-effects (clears frame IRQ latch).
    #[must_use]
    pub fn read_status(&mut self) -> u8 {
        let status = self.peek_status();
        self.frame_irq_pending = false;
        status
    }

    /// Reads status register without side-effects.
    #[must_use]
    pub fn peek_status(&self) -> u8 {
        let mut status = 0_u8;
        if self.pulse1.length_counter() > 0 {
            status |= 0x01;
        }
        if self.pulse2.length_counter() > 0 {
            status |= 0x02;
        }
        if self.triangle.length_counter() > 0 {
            status |= 0x04;
        }
        if self.noise.length_counter() > 0 {
            status |= 0x08;
        }
        if self.dmc.active() {
            status |= 0x10;
        }
        if self.frame_irq_pending {
            status |= 0x40;
        }
        if self.dmc.irq_pending() {
            status |= 0x80;
        }
        status
    }

    /// Returns total APU CPU-cycle ticks.
    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.cpu_cycles
    }

    /// Returns frame-sequencer quarter-frame tick count.
    #[must_use]
    pub fn quarter_frame_ticks(&self) -> u64 {
        self.quarter_frame_ticks
    }

    /// Returns frame-sequencer half-frame tick count.
    #[must_use]
    pub fn half_frame_ticks(&self) -> u64 {
        self.half_frame_ticks
    }

    /// Returns combined frame IRQ or DMC IRQ pending state.
    #[must_use]
    pub fn irq_pending(&self) -> bool {
        self.frame_irq_pending || self.dmc.irq_pending()
    }

    /// Returns DMC IRQ pending state.
    #[must_use]
    pub fn dmc_irq_pending(&self) -> bool {
        self.dmc.irq_pending()
    }

    /// Returns remaining bytes in active DMC sample playback.
    #[must_use]
    pub fn dmc_bytes_remaining(&self) -> u16 {
        self.dmc.bytes_remaining()
    }

    /// Returns total DMC fetch request count.
    #[must_use]
    pub fn dmc_fetch_count(&self) -> u64 {
        self.dmc.fetch_count()
    }

    /// Returns `(pulse1_timer_reload, pulse2_timer_reload)`.
    #[must_use]
    pub fn pulse_timer_reloads(&self) -> (u16, u16) {
        (self.pulse1.timer_reload, self.pulse2.timer_reload)
    }

    fn write_status(&mut self, value: u8) {
        self.pulse1.set_enabled(value & 0x01 != 0);
        self.pulse2.set_enabled(value & 0x02 != 0);
        self.triangle.set_enabled(value & 0x04 != 0);
        self.noise.set_enabled(value & 0x08 != 0);
        self.dmc.write_status_enable(value & 0x10 != 0);
        self.dmc.clear_irq();
    }

    fn write_frame_counter(&mut self, value: u8) {
        self.frame_mode_5 = value & 0x80 != 0;
        self.frame_irq_inhibit = value & 0x40 != 0;
        if self.frame_irq_inhibit {
            self.frame_irq_pending = false;
        }
        self.frame_cycle = 0;
        if self.frame_mode_5 {
            self.clock_quarter_frame();
            self.clock_half_frame();
        }
    }

    fn clock_timers(&mut self) -> Option<DmcDmaRequest> {
        self.triangle.clock_timer();
        if self.cpu_cycles.is_multiple_of(2) {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            self.noise.clock_timer();
        }
        self.dmc.step_timer()
    }

    fn clock_frame_sequencer(&mut self) {
        if self.frame_mode_5 {
            match self.frame_cycle {
                FRAME_STEP_1 | FRAME_STEP_3 => self.clock_quarter_frame(),
                FRAME_STEP_2 | FRAME_STEP_5 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    if self.frame_cycle == FRAME_STEP_5 {
                        self.frame_cycle = 0;
                    }
                }
                _ => {}
            }
            return;
        }

        match self.frame_cycle {
            FRAME_STEP_1 | FRAME_STEP_3 => self.clock_quarter_frame(),
            FRAME_STEP_2 => {
                self.clock_quarter_frame();
                self.clock_half_frame();
            }
            FRAME_STEP_4 => {
                self.clock_quarter_frame();
                self.clock_half_frame();
                if !self.frame_irq_inhibit {
                    self.frame_irq_pending = true;
                }
                self.frame_cycle = 0;
            }
            _ => {}
        }
    }

    fn clock_quarter_frame(&mut self) {
        self.quarter_frame_ticks = self.quarter_frame_ticks.saturating_add(1);
        self.pulse1.clock_quarter_frame();
        self.pulse2.clock_quarter_frame();
        self.triangle.clock_quarter_frame();
        self.noise.clock_quarter_frame();
    }

    fn clock_half_frame(&mut self) {
        self.half_frame_ticks = self.half_frame_ticks.saturating_add(1);
        self.pulse1.clock_half_frame();
        self.pulse2.clock_half_frame();
        self.triangle.clock_half_frame();
        self.noise.clock_half_frame();
    }

    fn raw_mixed_sample(&self, paused: bool) -> i16 {
        if paused {
            return 0;
        }

        let p1 = usize::from(self.pulse1.output());
        let p2 = usize::from(self.pulse2.output());
        let tri = usize::from(self.triangle.output());
        let noi = usize::from(self.noise.output());
        let dmc = usize::from(self.dmc.output());

        let (pulse_table, tnd_table) = get_mixer_tables();

        let pulse_sum = p1 + p2;
        let pulse_out = pulse_table[pulse_sum];

        let tnd_idx = (tri * 16 * 128) + (noi * 128) + dmc;
        let tnd_out = tnd_table[tnd_idx];

        let mut mixed = pulse_out + tnd_out;
        mixed = mixed.clamp(0.0, 1.0);

        (mixed * MAX_SAMPLE_AMPLITUDE) as i16
    }

    fn apply_output_filters(&mut self, raw_sample: i16) -> i16 {
        // NES hardware output path: high-pass 90 Hz, high-pass 440 Hz, then low-pass 14 kHz.
        let x_q16 = i64::from(raw_sample) << 16;

        let hp90 = (HPF_90_R_Q16
            * (self
                .hp90_prev_out_q16
                .saturating_add(x_q16)
                .saturating_sub(self.hp90_prev_in_q16)))
            >> 16;
        self.hp90_prev_in_q16 = x_q16;
        self.hp90_prev_out_q16 = hp90;

        let hp440 = (HPF_440_R_Q16
            * (self
                .hp440_prev_out_q16
                .saturating_add(hp90)
                .saturating_sub(self.hp440_prev_in_q16)))
            >> 16;
        self.hp440_prev_in_q16 = hp90;
        self.hp440_prev_out_q16 = hp440;

        let lp14k = self
            .lp14k_prev_out_q16
            .saturating_add((LPF_14K_A_Q16 * hp440.saturating_sub(self.lp14k_prev_out_q16)) >> 16);
        self.lp14k_prev_out_q16 = lp14k;

        let sample = lp14k >> 16;
        let sample = soft_limit_sample(sample);
        sample.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16
    }
}

fn soft_limit_sample(sample: i64) -> i64 {
    let abs = sample.unsigned_abs() as i64;
    if abs <= SOFT_CLIP_KNEE {
        return sample;
    }
    let excess = abs - SOFT_CLIP_KNEE;
    let compressed = SOFT_CLIP_KNEE + (excess >> SOFT_CLIP_RATIO_SHIFT);
    if sample.is_negative() {
        -compressed
    } else {
        compressed
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-computed APU mixer tables.
///
/// **Why this optimization matters:**
/// The original `raw_mixed_sample` method executed multiple floating-point divisions
/// and additions for every generated sample. Since the APU runs on every CPU cycle
/// (~1.78M times per second), eliminating f32 math and branches on the hot path
/// and replacing them with array lookups reduces a massive overhead and significantly
/// improves emulator performance.
static MIXER_TABLES: OnceLock<(Vec<f32>, Vec<f32>)> = OnceLock::new();

fn get_mixer_tables() -> &'static (Vec<f32>, Vec<f32>) {
    MIXER_TABLES.get_or_init(|| {
        let mut pulse = vec![0.0; 31];
        for (i, slot) in pulse.iter_mut().enumerate() {
            let pulse_sum = i as f32;
            *slot = if pulse_sum == 0.0 {
                0.0
            } else {
                95.88 / ((8128.0 / pulse_sum) + 100.0)
            };
        }

        let mut tnd = vec![0.0; 32768];
        for tri in 0..16 {
            for noi in 0..16 {
                for dmc in 0..128 {
                    let tnd_sum =
                        (tri as f32 / 8227.0) + (noi as f32 / 12241.0) + (dmc as f32 / 22638.0);
                    let tnd_out = if tnd_sum == 0.0 {
                        0.0
                    } else {
                        159.79 / ((1.0 / tnd_sum) + 100.0)
                    };
                    let idx = (tri * 16 * 128) + (noi * 128) + dmc;
                    tnd[idx] = tnd_out;
                }
            }
        }
        (pulse, tnd)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_control_write_does_not_restart_envelope() {
        let mut pulse = PulseChannel::new(true);
        pulse.envelope_start = false;
        pulse.write_control(0x1F);
        assert!(
            !pulse.envelope_start,
            "writing pulse control must not restart the envelope"
        );
    }

    #[test]
    fn noise_control_write_does_not_restart_envelope() {
        let mut noise = NoiseChannel::new();
        noise.envelope_start = false;
        noise.write_control(0x1F);
        assert!(
            !noise.envelope_start,
            "writing noise control must not restart the envelope"
        );
    }

    #[test]
    fn dmc_timer_uses_exact_period_cycles() {
        let mut dmc = DmcChannel::new();
        dmc.write_control(0x0F); // fastest rate => 54 CPU cycles
        dmc.timer_counter = dmc.rate_period_counter();
        let initial_bits = dmc.bits_remaining;

        for _ in 0..53 {
            dmc.step_timer();
            assert_eq!(dmc.bits_remaining, initial_bits);
        }

        dmc.step_timer();
        assert_eq!(dmc.bits_remaining, initial_bits - 1);
    }

    #[test]
    fn dmc_request_does_not_consume_sample_until_loaded() {
        let mut dmc = DmcChannel::new();
        dmc.write_control(0x8F); // IRQ enabled, no loop
        dmc.write_sample_addr(0x00);
        dmc.write_sample_len(0x00); // 1 byte sample
        dmc.write_status_enable(true);

        assert_eq!(dmc.bytes_remaining, 1);
        let request = dmc.step_timer();
        assert!(request.is_some(), "expected a DMA request");
        assert_eq!(
            dmc.bytes_remaining, 1,
            "bytes remaining must not decrement before DMA data arrives"
        );

        for _ in 0..8 {
            let follow_up = dmc.step_timer();
            assert!(follow_up.is_none(), "request should stay pending");
        }

        dmc.load_sample(0xAA);
        assert_eq!(dmc.bytes_remaining, 0);
        assert!(dmc.irq_pending, "finishing the sample should latch DMC IRQ");
    }
}
