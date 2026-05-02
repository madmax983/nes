import sys

def replace(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    search1 = """    fn advance_hardware_cycles(&mut self, cycles: u64) {
        // Batch-advance scheduler accounting counters once for the whole burst.
        // The counters are not observed mid-loop, so this is equivalent to N
        // per-cycle calls but avoids 5 wrapping_add operations per CPU cycle.
        // One CPU cycle = 1 APU cycle = 3 PPU cycles.
        self.scheduler.advance_by(cycles);

        for _ in 0..cycles {
            let dmc_request = self.apu.step_cpu_cycle(self.paused);
            for _ in 0..3 {
                self.ppu.step_dot();
                if let Some(mapper) = self.mapper.as_mut() {
                    mapper.on_ppu_dot(
                        self.ppu.scanline(),
                        self.ppu.dot(),
                        self.ppu.rendering_enabled_for_mapper_irq(),
                        self.ppu.ctrl(),
                    );
                }
            }
            if let Some(request) = dmc_request {
                self.apply_dmc_dma_request(request);
            }
        }
    }"""
    replace1 = """    fn step_hardware_cycle(&mut self) -> Option<DmcDmaRequest> {
        let dmc_request = self.apu.step_cpu_cycle(self.paused);
        for _ in 0..3 {
            self.ppu.step_dot();
            if let Some(mapper) = self.mapper.as_mut() {
                mapper.on_ppu_dot(
                    self.ppu.scanline(),
                    self.ppu.dot(),
                    self.ppu.rendering_enabled_for_mapper_irq(),
                    self.ppu.ctrl(),
                );
            }
        }
        dmc_request
    }

    fn advance_hardware_cycles(&mut self, cycles: u64) {
        // Batch-advance scheduler accounting counters once for the whole burst.
        // The counters are not observed mid-loop, so this is equivalent to N
        // per-cycle calls but avoids 5 wrapping_add operations per CPU cycle.
        // One CPU cycle = 1 APU cycle = 3 PPU cycles.
        self.scheduler.advance_by(cycles);

        for _ in 0..cycles {
            let dmc_request = self.step_hardware_cycle();
            if let Some(request) = dmc_request {
                self.apply_dmc_dma_request(request);
            }
        }
    }"""

    search2 = """    fn apply_dmc_dma_request(&mut self, request: DmcDmaRequest) {
        let sample = self.cpu.read_byte(request.addr);
        self.apu.load_dmc_sample(sample);
        for _ in 0..request.stall_cycles {
            self.scheduler.step_cpu_cycle();
            self.scheduler.step_apu_cycle();
            let dmc_request = self.apu.step_cpu_cycle(self.paused);
            for _ in 0..3 {
                self.scheduler.step_ppu_cycle();
                self.ppu.step_dot();
                if let Some(mapper) = self.mapper.as_mut() {
                    mapper.on_ppu_dot(
                        self.ppu.scanline(),
                        self.ppu.dot(),
                        self.ppu.rendering_enabled_for_mapper_irq(),
                        self.ppu.ctrl(),
                    );
                }
            }
            if let Some(chained) = dmc_request {
                let byte = self.cpu.read_byte(chained.addr);
                self.apu.load_dmc_sample(byte);
            }
        }
    }"""
    replace2 = """    fn apply_dmc_dma_request(&mut self, request: DmcDmaRequest) {
        let sample = self.cpu.read_byte(request.addr);
        self.apu.load_dmc_sample(sample);
        for _ in 0..request.stall_cycles {
            self.scheduler.step_cpu_cycle();
            self.scheduler.step_apu_cycle();
            let dmc_request = self.step_hardware_cycle();
            for _ in 0..3 {
                self.scheduler.step_ppu_cycle();
            }
            if let Some(chained) = dmc_request {
                let byte = self.cpu.read_byte(chained.addr);
                self.apu.load_dmc_sample(byte);
            }
        }
    }"""

    if search1 in content and search2 in content:
        content = content.replace(search1, replace1)
        content = content.replace(search2, replace2)
        with open(filepath, 'w') as f:
            f.write(content)
        print("Success")
    else:
        print("Search string not found")

replace('crates/nes-core/src/api.rs')
