import init, { NesWebEmulator } from "./pkg/nes_web.js";

const canvas = /** @type {HTMLCanvasElement} */ (document.getElementById("screen"));
const statusEl = /** @type {HTMLPreElement} */ (document.getElementById("status"));
const romFileInput = /** @type {HTMLInputElement} */ (document.getElementById("rom-file"));
const loadHomebrewBtn = /** @type {HTMLButtonElement} */ (document.getElementById("load-homebrew"));
const toggleRunBtn = /** @type {HTMLButtonElement} */ (document.getElementById("toggle-run"));
const stepFrameBtn = /** @type {HTMLButtonElement} */ (document.getElementById("step-frame"));
const resetBtn = /** @type {HTMLButtonElement} */ (document.getElementById("reset"));
const audioBtn = /** @type {HTMLButtonElement} */ (document.getElementById("audio"));

const ctx = canvas.getContext("2d", { alpha: false, desynchronized: true });
if (!ctx) {
  throw new Error("2D canvas context unavailable.");
}

let emu = null;
let romLoaded = false;
let running = false;
let rafId = null;
let tickCount = 0;

let audioContext = null;
let audioEnabled = false;
let audioNextStart = 0;

const imageData = ctx.createImageData(256, 240);

bootstrap().catch((err) => {
  setStatus(`Bootstrap failed:\n${stringifyError(err)}`);
});

async function bootstrap() {
  setStatus("Initializing wasm module...");
  await init();
  emu = new NesWebEmulator();
  setStatus("WASM ready.\nLoad a .nes ROM or click 'Load Homebrew ROM'.");
  requestTick();
}

romFileInput.addEventListener("change", async () => {
  const file = romFileInput.files?.[0];
  if (!file) {
    return;
  }
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    loadRomBytes(bytes, `Loaded ROM from file: ${file.name}`);
  } catch (err) {
    setStatus(`ROM file load failed:\n${stringifyError(err)}`);
  }
});

loadHomebrewBtn.addEventListener("click", async () => {
  try {
    const response = await fetch("/roms/homebrew/homebrew.nes");
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} while fetching /roms/homebrew/homebrew.nes`);
    }
    const bytes = new Uint8Array(await response.arrayBuffer());
    loadRomBytes(bytes, "Loaded /roms/homebrew/homebrew.nes");
  } catch (err) {
    setStatus(
      `Homebrew ROM fetch failed:\n${stringifyError(err)}\n` +
        "Build ROM first: cargo run -p nes-test-harness --bin build_homebrew_rom"
    );
  }
});

toggleRunBtn.addEventListener("click", () => {
  if (!romLoaded) {
    setStatus("Load a ROM before starting emulation.");
    return;
  }
  running = !running;
  syncRunButton();
  if (running) {
    requestTick();
  }
});

stepFrameBtn.addEventListener("click", () => {
  if (!emu || !romLoaded) {
    setStatus("Load a ROM before stepping.");
    return;
  }
  try {
    emu.step_frame();
    renderFrame();
    updateHud();
  } catch (err) {
    running = false;
    syncRunButton();
    setStatus(`StepFrame failed:\n${stringifyError(err)}`);
  }
});

resetBtn.addEventListener("click", () => {
  if (!emu || !romLoaded) {
    return;
  }
  try {
    emu.reset();
    renderFrame();
    updateHud();
  } catch (err) {
    setStatus(`Reset failed:\n${stringifyError(err)}`);
  }
});

audioBtn.addEventListener("click", async () => {
  if (!emu) {
    return;
  }
  try {
    if (!audioContext) {
      const sampleRate = emu.audio_sample_rate();
      audioContext = new AudioContext({ sampleRate });
    }
    await audioContext.resume();
    audioEnabled = true;
    audioNextStart = Math.max(audioContext.currentTime, audioNextStart);
    audioBtn.textContent = "Audio Enabled";
    setStatus("Audio enabled.");
  } catch (err) {
    setStatus(`Audio init failed:\n${stringifyError(err)}`);
  }
});

window.addEventListener("keydown", (event) => {
  if (!emu || !romLoaded) {
    return;
  }
  const mapped = dispatchKey(event.code, true);
  if (mapped) {
    event.preventDefault();
  }
});

window.addEventListener("keyup", (event) => {
  if (!emu || !romLoaded) {
    return;
  }
  const mapped = dispatchKey(event.code, false);
  if (mapped) {
    event.preventDefault();
  }
});

function requestTick() {
  if (rafId !== null) {
    return;
  }
  rafId = window.requestAnimationFrame(tick);
}

function tick() {
  rafId = null;
  if (!emu) {
    return;
  }

  if (running && romLoaded) {
    try {
      emu.step_frame();
      if (audioEnabled) {
        queueAudioChunk();
      }
    } catch (err) {
      running = false;
      syncRunButton();
      setStatus(`Runtime error:\n${stringifyError(err)}`);
    }
  }

  if (romLoaded) {
    renderFrame();
    tickCount += 1;
    if (tickCount % 20 === 0) {
      updateHud();
    }
  }

  if (running || romLoaded) {
    requestTick();
  }
}

function renderFrame() {
  if (!emu) {
    return;
  }
  const frame = emu.frame_rgba();
  if (frame.length !== imageData.data.length) {
    throw new Error(`unexpected frame length: ${frame.length}`);
  }
  imageData.data.set(frame);
  ctx.putImageData(imageData, 0, 0);
}

function loadRomBytes(bytes, message) {
  if (!emu) {
    throw new Error("WASM runtime not initialized.");
  }
  emu.load_rom(bytes);
  romLoaded = true;
  running = false;
  tickCount = 0;
  syncRunButton();
  renderFrame();
  updateHud(message);
}

function syncRunButton() {
  toggleRunBtn.textContent = running ? "Pause" : "Start";
  toggleRunBtn.setAttribute("aria-pressed", running ? "true" : "false");
}

function updateHud(prefix = "") {
  if (!emu) {
    return;
  }
  const lines = [];
  if (prefix) {
    lines.push(prefix);
    lines.push("");
  }
  lines.push(`state: ${running ? "running" : "paused"}`);
  lines.push(`ppu_frame: ${emu.ppu_frame_counter()}`);
  lines.push(`pc: $${emu.cpu_pc().toString(16).toUpperCase().padStart(4, "0")}`);
  lines.push(`fps_target: ${(emu.fps_milli() / 1000).toFixed(1)}`);
  lines.push(`controller_bits: 0x${emu.controller_bits().toString(16).toUpperCase().padStart(2, "0")}`);
  lines.push(`audio: ${audioEnabled ? "enabled" : "disabled"}`);
  lines.push("");
  lines.push("controls:");
  lines.push("arrows=dpad  z=a  x=b  enter=start  right-shift=select");
  lines.push("load a ROM before starting");
  statusEl.textContent = lines.join("\n");
}

function setStatus(text) {
  statusEl.textContent = text;
}

function stringifyError(err) {
  if (err instanceof Error) {
    return err.stack ?? err.message;
  }
  return String(err);
}

function dispatchKey(code, pressed) {
  if (!emu) {
    return false;
  }
  try {
    return emu.dispatch_dom_key(code, pressed);
  } catch (err) {
    setStatus(`Input dispatch failed:\n${stringifyError(err)}`);
    return false;
  }
}

function queueAudioChunk() {
  if (!emu || !audioContext || audioContext.state !== "running") {
    return;
  }
  const chunk = emu.audio_chunk_i16();
  if (chunk.length === 0) {
    return;
  }

  const buffer = audioContext.createBuffer(1, chunk.length, emu.audio_sample_rate());
  const samples = buffer.getChannelData(0);
  for (let idx = 0; idx < chunk.length; idx += 1) {
    samples[idx] = Math.max(-1, Math.min(1, chunk[idx] / 32768));
  }

  const source = audioContext.createBufferSource();
  source.buffer = buffer;
  source.connect(audioContext.destination);

  const lead = 0.02;
  const startAt = Math.max(audioContext.currentTime + lead, audioNextStart);
  source.start(startAt);
  audioNextStart = startAt + buffer.duration;

  if (audioNextStart - audioContext.currentTime > 0.45) {
    audioNextStart = audioContext.currentTime + 0.2;
  }
}
