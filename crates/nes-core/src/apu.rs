use std::collections::VecDeque;

use crate::api::AUDIO_SAMPLE_RATE;

const CPU_CLOCK_HZ: u64 = 1_789_773;
const MAX_SAMPLE_AMPLITUDE: i16 = 12_000;
const MAX_QUEUED_SAMPLES: usize = AUDIO_SAMPLE_RATE as usize;

const FRAME_STEP_1: u16 = 3_729;
const FRAME_STEP_2: u16 = 7_457;
const FRAME_STEP_3: u16 = 11_186;
const FRAME_STEP_4: u16 = 14_916;
const FRAME_STEP_5: u16 = 18_641;

const DUTY_THRESHOLDS: [u32; 4] = [0x2000_0000, 0x4000_0000, 0x8000_0000, 0xC000_0000];

const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApuSnapshot {
    pub cpu_cycles: u64,
    pub frame_cycle: u16,
    pub quarter_frame_ticks: u64,
    pub half_frame_ticks: u64,
    pub irq_pending: bool,
    pub frame_irq_inhibit: bool,
    pub frame_mode_5: bool,
    pub pulse_timer: u16,
    pub pulse_duty: u8,
    pub length_counter: u8,
    pub tone_phase: u32,
    pub sample_accumulator: u64,
    pub samples: Vec<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apu {
    cpu_cycles: u64,
    frame_cycle: u16,
    quarter_frame_ticks: u64,
    half_frame_ticks: u64,
    irq_pending: bool,
    frame_irq_inhibit: bool,
    frame_mode_5: bool,
    pulse_ctrl: u8,
    pulse_timer_low: u8,
    pulse_timer_high: u8,
    pulse_timer: u16,
    pulse_duty: u8,
    length_counter: u8,
    tone_phase: u32,
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
            irq_pending: false,
            frame_irq_inhibit: false,
            frame_mode_5: false,
            pulse_ctrl: 0,
            pulse_timer_low: 0,
            pulse_timer_high: 0,
            pulse_timer: 0,
            pulse_duty: 2,
            length_counter: LENGTH_TABLE[0],
            tone_phase: 0,
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
            irq_pending: self.irq_pending,
            frame_irq_inhibit: self.frame_irq_inhibit,
            frame_mode_5: self.frame_mode_5,
            pulse_timer: self.pulse_timer,
            pulse_duty: self.pulse_duty,
            length_counter: self.length_counter,
            tone_phase: self.tone_phase,
            sample_accumulator: self.sample_accumulator,
            samples: self.samples.iter().copied().collect(),
        }
    }

    pub fn restore(&mut self, snapshot: ApuSnapshot) {
        self.cpu_cycles = snapshot.cpu_cycles;
        self.frame_cycle = snapshot.frame_cycle;
        self.quarter_frame_ticks = snapshot.quarter_frame_ticks;
        self.half_frame_ticks = snapshot.half_frame_ticks;
        self.irq_pending = snapshot.irq_pending;
        self.frame_irq_inhibit = snapshot.frame_irq_inhibit;
        self.frame_mode_5 = snapshot.frame_mode_5;
        self.pulse_timer = snapshot.pulse_timer;
        self.pulse_duty = snapshot.pulse_duty.min(3);
        self.length_counter = snapshot.length_counter;
        self.tone_phase = snapshot.tone_phase;
        self.sample_accumulator = snapshot.sample_accumulator;
        self.samples = snapshot.samples.into_iter().collect();
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000 => {
                self.pulse_ctrl = value;
                self.pulse_duty = (value >> 6) & 0x03;
            }
            0x4002 => {
                self.pulse_timer_low = value;
                self.refresh_pulse_timer();
            }
            0x4003 => {
                self.pulse_timer_high = value & 0x07;
                self.refresh_pulse_timer();
                let index = (value >> 3) as usize & 0x1F;
                self.length_counter = LENGTH_TABLE[index];
                self.tone_phase = 0;
            }
            0x4015 => {
                self.irq_pending = false;
                if value & 0x01 == 0 {
                    self.length_counter = 0;
                }
            }
            0x4017 => {
                self.frame_mode_5 = value & 0x80 != 0;
                self.frame_irq_inhibit = value & 0x40 != 0;
                if self.frame_irq_inhibit {
                    self.irq_pending = false;
                }
                self.frame_cycle = 0;
            }
            _ => {}
        }
    }

    pub fn step_cpu_cycle(&mut self, controller_bits: u8, paused: bool, ppu_in_vblank: bool) {
        self.cpu_cycles = self.cpu_cycles.saturating_add(1);
        self.frame_cycle = self.frame_cycle.saturating_add(1);
        self.clock_frame_sequencer();

        let sample = self.mixed_sample(controller_bits, paused, ppu_in_vblank, CPU_CLOCK_HZ);
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
            drained.push(self.mixed_sample(
                controller_bits,
                paused,
                ppu_in_vblank,
                u64::from(AUDIO_SAMPLE_RATE),
            ));
        }
        drained
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
        self.irq_pending
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
                    self.irq_pending = true;
                }
                self.frame_cycle = 0;
            }
            _ => {}
        }
    }

    fn clock_quarter_frame(&mut self) {
        self.quarter_frame_ticks = self.quarter_frame_ticks.saturating_add(1);
    }

    fn clock_half_frame(&mut self) {
        self.half_frame_ticks = self.half_frame_ticks.saturating_add(1);
        let length_halt = self.pulse_ctrl & 0x20 != 0;
        if !length_halt && self.length_counter > 0 {
            self.length_counter = self.length_counter.saturating_sub(1);
        }
    }

    fn refresh_pulse_timer(&mut self) {
        self.pulse_timer =
            (u16::from(self.pulse_timer_high) << 8) | u16::from(self.pulse_timer_low);
    }

    fn mixed_sample(
        &mut self,
        controller_bits: u8,
        paused: bool,
        ppu_in_vblank: bool,
        phase_clock_hz: u64,
    ) -> i16 {
        let freq_hz = self.effective_pulse_frequency(controller_bits);
        let phase_step = ((u64::from(freq_hz) << 32) / phase_clock_hz) as u32;
        self.tone_phase = self.tone_phase.wrapping_add(phase_step);

        let duty_index = usize::from(self.pulse_duty.min(3));
        let threshold = DUTY_THRESHOLDS[duty_index];
        let high = self.tone_phase < threshold;

        let mut amplitude = if paused || self.length_counter == 0 {
            0
        } else {
            MAX_SAMPLE_AMPLITUDE
        };
        if ppu_in_vblank {
            amplitude /= 2;
        }
        if controller_bits & 0b0000_0010 != 0 {
            amplitude = amplitude.saturating_add(MAX_SAMPLE_AMPLITUDE / 4);
        }

        if high { amplitude } else { -amplitude }
    }

    fn effective_pulse_frequency(&self, controller_bits: u8) -> u32 {
        let timer = self.pulse_timer.max(8);
        let hw_freq = CPU_CLOCK_HZ / (16 * u64::from(timer.saturating_add(1)));
        let mut freq = u32::try_from(hw_freq).unwrap_or(110);
        freq = freq.clamp(55, 1_760);
        let button_bias = controller_bits.count_ones().saturating_mul(17);
        freq.saturating_add(button_bias).min(1_760)
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}
