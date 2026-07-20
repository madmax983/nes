use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nes_core::NesCore;
use nes_netplay::{RollbackConfig, RollbackEngine};
use nes_test_harness::build_homebrew_rom;
use serde::Serialize;

const BUTTON_A: u8 = 0x01;
const BUTTON_B: u8 = 0x02;
const BUTTON_UP: u8 = 0x10;
const BUTTON_RIGHT: u8 = 0x80;
const MAX_LOG_ENTRIES: usize = 512;

#[derive(Debug, Clone, Copy, Serialize)]
struct SimulatedNetworkConfig {
    latency_frames: u64,
    jitter_frames: u64,
    loss_pct: u8,
    reorder_pct: u8,
}

#[derive(Debug, Clone, Serialize)]
struct PacketRecord {
    event: &'static str,
    host_frame: u64,
    from_player: u8,
    to_player: u8,
    input_frame: u64,
    bits: u8,
    deliver_on: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct StepRecord {
    host_frame: u64,
    emu_frame: u64,
    peer1_hash: u64,
    peer2_hash: u64,
    peer1_pc: u16,
    peer2_pc: u16,
    peer1_rollback_distance: u64,
    peer2_rollback_distance: u64,
}

#[derive(Debug, Clone)]
struct InFlightPacket {
    from_player: u8,
    to_player: u8,
    input_frame: u64,
    bits: u8,
    deliver_on: u64,
    sequence: u64,
}

struct SimulatedNetwork {
    config: SimulatedNetworkConfig,
    rng_state: u64,
    sequence: u64,
    in_flight: Vec<InFlightPacket>,
    log: VecDeque<PacketRecord>,
}

impl SimulatedNetwork {
    fn new(config: SimulatedNetworkConfig, seed: u64) -> Self {
        Self {
            config,
            rng_state: seed.max(1),
            sequence: 0,
            in_flight: Vec::new(),
            log: VecDeque::new(),
        }
    }

    fn send_input(
        &mut self,
        host_frame: u64,
        from_player: u8,
        to_player: u8,
        input_frame: u64,
        bits: u8,
    ) {
        if self.percent_hit(self.config.loss_pct) {
            self.push_log(PacketRecord {
                event: "dropped",
                host_frame,
                from_player,
                to_player,
                input_frame,
                bits,
                deliver_on: None,
            });
            return;
        }

        let delay = self.sample_delay_frames();
        let deliver_on = host_frame.saturating_add(delay);
        let packet = InFlightPacket {
            from_player,
            to_player,
            input_frame,
            bits,
            deliver_on,
            sequence: self.sequence,
        };
        self.sequence = self.sequence.saturating_add(1);
        self.push_log(PacketRecord {
            event: "queued",
            host_frame,
            from_player,
            to_player,
            input_frame,
            bits,
            deliver_on: Some(deliver_on),
        });
        self.in_flight.push(packet);
        self.in_flight
            .sort_by_key(|in_flight| (in_flight.deliver_on, in_flight.sequence));
    }

    fn drain_ready(&mut self, host_frame: u64) -> Vec<InFlightPacket> {
        let mut ready = Vec::new();
        let mut pending = Vec::with_capacity(self.in_flight.len());
        for packet in self.in_flight.drain(..) {
            if packet.deliver_on <= host_frame {
                ready.push(packet);
            } else {
                pending.push(packet);
            }
        }
        pending.sort_by_key(|in_flight| (in_flight.deliver_on, in_flight.sequence));
        self.in_flight = pending;
        for packet in &ready {
            self.push_log(PacketRecord {
                event: "delivered",
                host_frame,
                from_player: packet.from_player,
                to_player: packet.to_player,
                input_frame: packet.input_frame,
                bits: packet.bits,
                deliver_on: Some(packet.deliver_on),
            });
        }
        ready
    }

    fn recent_log(&self) -> Vec<PacketRecord> {
        self.log.iter().cloned().collect()
    }

    fn sample_delay_frames(&mut self) -> u64 {
        let mut delay = self.config.latency_frames as i64;
        if self.config.jitter_frames > 0 {
            let span = self
                .config
                .jitter_frames
                .saturating_mul(2)
                .saturating_add(1);
            let offset = (self.next_u64() % span) as i64 - self.config.jitter_frames as i64;
            delay = delay.saturating_add(offset);
        }
        if self.percent_hit(self.config.reorder_pct) {
            let extra = self.config.latency_frames.max(1)
                + (self.next_u64() % (self.config.jitter_frames + 1));
            let extra_i64 = i64::try_from(extra).unwrap_or(i64::MAX);
            delay = delay.saturating_add(extra_i64);
        }
        if delay <= 0 { 0 } else { delay as u64 }
    }

    fn percent_hit(&mut self, pct: u8) -> bool {
        match pct {
            0 => false,
            100 => true,
            value => (self.next_u64() % 100) < u64::from(value),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.rng_state
    }

    fn push_log(&mut self, record: PacketRecord) {
        if self.log.len() >= MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(record);
    }
}

struct PeerRuntime {
    core: NesCore,
    rollback: RollbackEngine,
}

impl PeerRuntime {
    fn new(local_player: u8, rom_bytes: &[u8], input_delay_frames: u32, max_rollback: u32) -> Self {
        let mut core = NesCore::new();
        core.load_ines_rom(rom_bytes)
            .expect("homebrew ROM should load for netplay soak");
        let rollback = RollbackEngine::new(RollbackConfig {
            local_player,
            input_delay_frames,
            max_rollback_frames: max_rollback,
        })
        .expect("rollback config should be valid");
        Self { core, rollback }
    }
}

#[derive(Debug, Serialize)]
struct DesyncReproBundle {
    reason: String,
    host_frame: u64,
    peer1_hash: u64,
    peer2_hash: u64,
    peer1_pc: u16,
    peer2_pc: u16,
    peer1_controller_bits: u8,
    peer2_controller_bits: u8,
    network: SimulatedNetworkConfig,
    recent_packets: Vec<PacketRecord>,
    recent_steps: Vec<StepRecord>,
}

#[test]
fn rollback_two_peer_soak_converges_under_faulty_network() {
    const ACTIVE_FRAMES: u64 = 1_200;
    const SETTLE_FRAMES: u64 = 260;
    const SETTLE_GUARD_FRAMES: u64 = 120;
    const INPUT_DELAY_FRAMES: u32 = 2;
    const MAX_ROLLBACK_FRAMES: u32 = 240;

    let rom = build_homebrew_rom().expect("homebrew ROM should assemble");
    let mut peer1 = PeerRuntime::new(1, &rom, INPUT_DELAY_FRAMES, MAX_ROLLBACK_FRAMES);
    let mut peer2 = PeerRuntime::new(2, &rom, INPUT_DELAY_FRAMES, MAX_ROLLBACK_FRAMES);
    let network_config = SimulatedNetworkConfig {
        latency_frames: 3,
        jitter_frames: 2,
        loss_pct: 0,
        reorder_pct: 35,
    };
    let mut network = SimulatedNetwork::new(network_config, 0xDEAD_BEEF_CAFE_F00D);
    let mut recent_steps = VecDeque::<StepRecord>::new();
    let mut rollback_events = 0_u64;

    for host_frame in 0..(ACTIVE_FRAMES + SETTLE_FRAMES) {
        let bits1 = scripted_input_bits(1, host_frame, ACTIVE_FRAMES);
        let bits2 = scripted_input_bits(2, host_frame, ACTIVE_FRAMES);

        let scheduled1 = peer1.rollback.schedule_local_input(bits1);
        let scheduled2 = peer2.rollback.schedule_local_input(bits2);
        network.send_input(host_frame, 1, 2, scheduled1.frame, scheduled1.bits);
        network.send_input(host_frame, 2, 1, scheduled2.frame, scheduled2.bits);

        for packet in network.drain_ready(host_frame) {
            let peer = if packet.to_player == 1 {
                &mut peer1
            } else {
                &mut peer2
            };
            let _ = peer
                .rollback
                .ingest_remote_input(packet.input_frame, packet.bits);
        }

        let step1 = peer1
            .rollback
            .advance_frame(&mut peer1.core)
            .expect("peer1 rollback step should succeed");
        let step2 = peer2
            .rollback
            .advance_frame(&mut peer2.core)
            .expect("peer2 rollback step should succeed");
        if step1.rollback_distance > 0 {
            rollback_events = rollback_events.saturating_add(1);
        }
        if step2.rollback_distance > 0 {
            rollback_events = rollback_events.saturating_add(1);
        }

        if recent_steps.len() >= MAX_LOG_ENTRIES {
            recent_steps.pop_front();
        }
        recent_steps.push_back(StepRecord {
            host_frame,
            emu_frame: step1.frame,
            peer1_hash: step1.state_hash,
            peer2_hash: step2.state_hash,
            peer1_pc: peer1.core.cpu_pc(),
            peer2_pc: peer2.core.cpu_pc(),
            peer1_rollback_distance: step1.rollback_distance,
            peer2_rollback_distance: step2.rollback_distance,
        });

        if host_frame >= ACTIVE_FRAMES + SETTLE_GUARD_FRAMES && step1.state_hash != step2.state_hash
        {
            let bundle = DesyncReproBundle {
                reason: "state hash mismatch after settle window".to_owned(),
                host_frame,
                peer1_hash: step1.state_hash,
                peer2_hash: step2.state_hash,
                peer1_pc: peer1.core.cpu_pc(),
                peer2_pc: peer2.core.cpu_pc(),
                peer1_controller_bits: peer1.core.controller_bits(),
                peer2_controller_bits: peer2.core.controller_bits(),
                network: network_config,
                recent_packets: network.recent_log(),
                recent_steps: recent_steps.iter().cloned().collect(),
            };
            let path = write_repro_bundle(&bundle);
            panic!(
                "rollback desync at host frame {} (hashes {:016X} != {:016X}); repro={}",
                host_frame,
                step1.state_hash,
                step2.state_hash,
                path.display()
            );
        }
    }

    assert_eq!(
        peer1.core.state_hash(),
        peer2.core.state_hash(),
        "final hashes should converge after settle period"
    );
    assert!(
        rollback_events > 0,
        "expected rollback activity under latency/jitter/reorder simulation"
    );
}

fn scripted_input_bits(player: u8, host_frame: u64, active_frames: u64) -> u8 {
    if host_frame >= active_frames {
        return 0;
    }
    let phase = if player == 1 {
        host_frame
    } else {
        host_frame.saturating_add(23)
    };

    let mut bits = BUTTON_RIGHT;
    if phase % 47 < 5 {
        bits |= BUTTON_A;
    }
    if phase % 71 < 4 {
        bits |= BUTTON_B;
    }
    if player == 2 && phase % 89 < 3 {
        bits |= BUTTON_UP;
    }
    bits
}

fn write_repro_bundle(bundle: &DesyncReproBundle) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = std::env::temp_dir().join(format!(
        "nes-netplay-repro-{}-{}.json",
        std::process::id(),
        timestamp
    ));
    let payload = serde_json::to_vec_pretty(bundle).expect("serialize repro bundle");
    fs::write(&path, payload).expect("write repro bundle");
    path
}
