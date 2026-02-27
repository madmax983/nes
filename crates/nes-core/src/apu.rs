use std::collections::VecDeque;

use crate::api::AUDIO_SAMPLE_RATE;

const CPU_CLOCK_HZ: u64 = 1_789_773;
const MAX_SAMPLE_AMPLITUDE: f32 = 12_000.0;
const MAX_QUEUED_SAMPLES: usize = AUDIO_SAMPLE_RATE as usize;

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
pub struct DmcDmaRequest {
    pub addr: u16,
    pub stall_cycles: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PulseChannel {
    enabled: bool,
    control: u8,
    timer_reload: u16,
    timer_counter: u16,
    duty_step: u8,
    length_counter: u8,
    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
}

impl PulseChannel {
    fn new() -> Self {
        Self {
            enabled: false,
            control: 0x30,
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
        let mut channel = Self::new();
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
        self.envelope_start = true;
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
    }

    fn output(&self) -> u8 {
        if self.length_counter == 0 || self.timer_reload < 8 {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        self.envelope_start = true;
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
            timer_counter: DMC_RATE_TABLE[0],
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
            self.bytes_remaining = 0;
        }
    }

    fn load_sample(&mut self, sample: u8) {
        self.sample_buffer = Some(sample);
    }

    fn step_timer(&mut self) -> Option<DmcDmaRequest> {
        if self.timer_counter == 0 {
            self.timer_counter = self.rate_period();
            self.clock_output();
        } else {
            self.timer_counter = self.timer_counter.saturating_sub(1);
        }

        if self.sample_buffer.is_none() && self.bytes_remaining > 0 {
            let request = DmcDmaRequest {
                addr: self.current_addr,
                stall_cycles: 4,
            };
            self.fetch_count = self.fetch_count.saturating_add(1);
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

    fn rate_period(&self) -> u16 {
        DMC_RATE_TABLE[usize::from(self.rate_index)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    samples: Vec<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    samples: VecDeque<i16>,
}

impl Apu {
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
            pulse2: PulseChannel::new(),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            sample_accumulator: 0,
            samples: VecDeque::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

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
            pulse1: self.pulse1.clone(),
            pulse2: self.pulse2.clone(),
            triangle: self.triangle.clone(),
            noise: self.noise.clone(),
            dmc: self.dmc.clone(),
            sample_accumulator: self.sample_accumulator,
            samples: self.samples.iter().copied().collect(),
        }
    }

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
        self.samples = snapshot.samples.into_iter().collect();
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000 => self.pulse1.write_control(value),
            0x4002 => self.pulse1.write_timer_low(value),
            0x4003 => self.pulse1.write_timer_high(value),
            0x4004 => self.pulse2.write_control(value),
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

    pub fn step_cpu_cycle(
        &mut self,
        controller_bits: u8,
        paused: bool,
        ppu_in_vblank: bool,
    ) -> Option<DmcDmaRequest> {
        self.cpu_cycles = self.cpu_cycles.saturating_add(1);
        self.frame_cycle = self.frame_cycle.saturating_add(1);

        self.clock_frame_sequencer();
        let dmc_request = self.clock_timers();

        let sample = self.mixed_sample(controller_bits, paused, ppu_in_vblank);
        self.sample_accumulator = self
            .sample_accumulator
            .saturating_add(u64::from(AUDIO_SAMPLE_RATE));
        while self.sample_accumulator >= CPU_CLOCK_HZ {
            self.sample_accumulator -= CPU_CLOCK_HZ;
            self.samples.push_back(sample);
            if self.samples.len() > MAX_QUEUED_SAMPLES {
                let _ = self.samples.pop_front();
            }
        }

        dmc_request
    }

    pub fn load_dmc_sample(&mut self, sample: u8) {
        self.dmc.load_sample(sample);
    }

    #[must_use]
    pub fn drain_samples(
        &mut self,
        count: usize,
        controller_bits: u8,
        paused: bool,
        ppu_in_vblank: bool,
    ) -> Vec<i16> {
        let mut drained = Vec::with_capacity(count);
        while drained.len() < count {
            if let Some(sample) = self.samples.pop_front() {
                drained.push(sample);
                continue;
            }
            if self
                .step_cpu_cycle(controller_bits, paused, ppu_in_vblank)
                .is_some()
            {
                // No CPU bus callback when called from output fetch path.
                self.dmc.load_sample(0);
            }
        }
        drained
    }

    #[must_use]
    pub fn read_status(&mut self) -> u8 {
        let status = self.peek_status();
        self.frame_irq_pending = false;
        status
    }

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

    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.cpu_cycles
    }

    #[must_use]
    pub fn quarter_frame_ticks(&self) -> u64 {
        self.quarter_frame_ticks
    }

    #[must_use]
    pub fn half_frame_ticks(&self) -> u64 {
        self.half_frame_ticks
    }

    #[must_use]
    pub fn irq_pending(&self) -> bool {
        self.frame_irq_pending || self.dmc.irq_pending()
    }

    #[must_use]
    pub fn dmc_irq_pending(&self) -> bool {
        self.dmc.irq_pending()
    }

    #[must_use]
    pub fn dmc_bytes_remaining(&self) -> u16 {
        self.dmc.bytes_remaining()
    }

    #[must_use]
    pub fn dmc_fetch_count(&self) -> u64 {
        self.dmc.fetch_count()
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

    fn mixed_sample(&self, controller_bits: u8, paused: bool, ppu_in_vblank: bool) -> i16 {
        if paused {
            return 0;
        }

        let p1 = f32::from(self.pulse1.output());
        let p2 = f32::from(self.pulse2.output());
        let tri = f32::from(self.triangle.output());
        let noi = f32::from(self.noise.output());
        let dmc = f32::from(self.dmc.output());

        let pulse_sum = p1 + p2;
        let pulse_out = if pulse_sum == 0.0 {
            0.0
        } else {
            95.88 / ((8128.0 / pulse_sum) + 100.0)
        };

        let tnd_sum = (tri / 8227.0) + (noi / 12241.0) + (dmc / 22638.0);
        let tnd_out = if tnd_sum == 0.0 {
            0.0
        } else {
            159.79 / ((1.0 / tnd_sum) + 100.0)
        };

        let mut mixed = pulse_out + tnd_out;
        mixed *= 1.0 + (controller_bits.count_ones() as f32 * 0.01);
        if ppu_in_vblank {
            mixed *= 0.75;
        }
        mixed = mixed.clamp(0.0, 1.0);

        (mixed * MAX_SAMPLE_AMPLITUDE) as i16
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}
