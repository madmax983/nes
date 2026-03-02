import init, { NesWebEmulator } from "./nes-web.js";
import {
  clearSavedRom,
  loadSavedRom,
  saveRomForOfflineUse,
  supportsRomStorage,
} from "./rom-storage.mjs";

const canvas = /** @type {HTMLCanvasElement} */ (document.getElementById("screen"));
const statusEl = /** @type {HTMLPreElement} */ (document.getElementById("status"));
const romFileInput = /** @type {HTMLInputElement} */ (document.getElementById("rom-file"));
const loadHomebrewBtn = /** @type {HTMLButtonElement} */ (document.getElementById("load-homebrew"));
const forgetSavedRomBtn = /** @type {HTMLButtonElement} */ (document.getElementById("forget-saved-rom"));
const toggleRunBtn = /** @type {HTMLButtonElement} */ (document.getElementById("toggle-run"));
const stepFrameBtn = /** @type {HTMLButtonElement} */ (document.getElementById("step-frame"));
const resetBtn = /** @type {HTMLButtonElement} */ (document.getElementById("reset"));
const audioBtn = /** @type {HTMLButtonElement} */ (document.getElementById("audio"));
const holdRightBtn = /** @type {HTMLButtonElement} */ (document.getElementById("hold-right"));
const captureBtn = /** @type {HTMLButtonElement} */ (document.getElementById("capture"));

const ctx = canvas.getContext("2d", { alpha: false, desynchronized: true });
if (!ctx) {
  throw new Error("2D canvas context unavailable.");
}

let emu = null;
let romLoaded = false;
let running = false;
let rafId = null;
let tickCount = 0;
let lastTickTimestampMs = null;
let frameBudgetMs = 0;

const MAX_CATCHUP_FRAMES = 4;
const MAX_DELTA_MS = 250;

const BUTTON_MASK = Object.freeze({
  A: 0x01,
  B: 0x02,
  SELECT: 0x04,
  START: 0x08,
  UP: 0x10,
  DOWN: 0x20,
  LEFT: 0x40,
  RIGHT: 0x80,
});

const KEY_TO_MASK = new Map([
  ["KeyZ", BUTTON_MASK.A],
  ["KeyX", BUTTON_MASK.B],
  ["KeyW", BUTTON_MASK.UP],
  ["KeyA", BUTTON_MASK.LEFT],
  ["KeyS", BUTTON_MASK.DOWN],
  ["KeyD", BUTTON_MASK.RIGHT],
  ["Enter", BUTTON_MASK.START],
  ["Tab", BUTTON_MASK.SELECT],
  ["KeyC", BUTTON_MASK.SELECT],
  ["ArrowUp", BUTTON_MASK.UP],
  ["ArrowDown", BUTTON_MASK.DOWN],
  ["ArrowLeft", BUTTON_MASK.LEFT],
  ["ArrowRight", BUTTON_MASK.RIGHT],
]);

let wallFps = 0;
let wallFpsFrameCount = 0;
let wallFpsWindowStartMs = 0;
let emuFramesStepped = 0;
let emuFps = 0;
let emuFpsFrameCount = 0;
let emuFpsWindowStartMs = 0;

let lastStateHash = 0;
let unchangedStateFrames = 0;
let activeControllerBits = 0;
let lastInputEvent = "none";
let inputEventCounter = 0;
let lastPc = 0;
let pcStallFrames = 0;
let autoCapturedStall = false;
let waitLoopFrames = 0;
let autoCaptureAttempts = 0;
let lastAutoCaptureReason = "none";
let lockedControllerBits = 0;

let audioContext = null;
let audioEnabled = false;
let audioWorkletNode = null;
let audioWorkletReady = false;
let audioQueuedSamples = 0;
let audioQueueMs = 0;
let hasSavedRom = false;

const AUDIO_MAX_QUEUE_MS = 35;
const AUDIO_TARGET_QUEUE_MS = 16;
const AUTO_CAPTURE_PC_STALL_FRAMES = 180;
const AUTO_CAPTURE_WAIT_LOOP_FRAMES = 120;
const SMB_WAIT_PC_MIN = 0x8150;
const SMB_WAIT_PC_MAX = 0x8155;

const imageData = ctx.createImageData(256, 240);

bootstrap().catch((err) => {
  setStatus(`Bootstrap failed:\n${stringifyError(err)}`);
});

async function bootstrap() {
  setStatus("Initializing wasm module...");
  await init();
  emu = new NesWebEmulator();
  const now = performance.now();
  wallFpsWindowStartMs = now;
  emuFpsWindowStartMs = now;
  const restored = await tryRestoreSavedRom();
  if (!restored) {
    setStatus("WASM ready.\nLoad a .nes ROM or click 'Load Homebrew ROM'.");
  }
  syncForgetSavedRomButton();
  canvas.focus();
  requestTick();
}

romFileInput.addEventListener("change", async () => {
  const file = romFileInput.files?.[0];
  if (!file) {
    return;
  }
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    await loadRomBytesWithPersistence(
      bytes,
      file.name,
      "file",
      `Loaded ROM from file: ${file.name}`,
    );
  } catch (err) {
    setStatus(`ROM file load failed:\n${stringifyError(err)}`);
  }
  romFileInput.blur();
  canvas.focus();
});

loadHomebrewBtn.addEventListener("click", async () => {
  try {
    let response = await fetch("./homebrew.nes");
    if (!response.ok) {
      response = await fetch("/roms/homebrew/homebrew.nes");
    }
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} while fetching homebrew ROM`);
    }
    const bytes = new Uint8Array(await response.arrayBuffer());
    await loadRomBytesWithPersistence(
      bytes,
      "homebrew.nes",
      "homebrew",
      "Loaded homebrew ROM",
    );
  } catch (err) {
    setStatus(
      `Homebrew ROM fetch failed:\n${stringifyError(err)}\n` +
        "Build ROM first: cargo run -p nes-test-harness --bin build_homebrew_rom"
    );
  }
});

forgetSavedRomBtn.addEventListener("click", async () => {
  if (!supportsRomStorage()) {
    setStatus("This browser does not support IndexedDB ROM storage.");
    return;
  }
  try {
    await clearSavedRom();
    hasSavedRom = false;
    syncForgetSavedRomButton();
    if (romLoaded) {
      updateHud("Cleared saved ROM from this device.");
    } else {
      setStatus("Cleared saved ROM from this device.");
    }
  } catch (err) {
    setStatus(`Failed to clear saved ROM:\n${stringifyError(err)}`);
  }
  canvas.focus();
});

toggleRunBtn.addEventListener("click", () => {
  if (!romLoaded) {
    setStatus("Load a ROM before starting emulation.");
    return;
  }
  running = !running;
  if (running) {
    lastTickTimestampMs = null;
  }
  syncRunButton();
  canvas.focus();
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
    emuFramesStepped += 1;
    observeStateProgress();
    renderFrame();
    updateHud();
    canvas.focus();
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
    clearControllerState();
    clearControllerLocks();
    resetTimingAndMetrics();
    renderFrame();
    updateHud();
    canvas.focus();
  } catch (err) {
    setStatus(`Reset failed:\n${stringifyError(err)}`);
  }
});

captureBtn.addEventListener("click", () => {
  captureFrameAndHud("manual");
  canvas.focus();
});

holdRightBtn.addEventListener("click", () => {
  if ((lockedControllerBits & BUTTON_MASK.RIGHT) !== 0) {
    lockedControllerBits &= ~BUTTON_MASK.RIGHT;
  } else {
    lockedControllerBits |= BUTTON_MASK.RIGHT;
  }
  syncHoldRightButton();
  applyControllerState();
  updateHud();
  canvas.focus();
});

audioBtn.addEventListener("click", async () => {
  if (!emu) {
    return;
  }
  try {
    await ensureAudioPipeline(emu.audio_sample_rate());
    await audioContext.resume();
    audioEnabled = true;
    audioQueuedSamples = 0;
    audioQueueMs = 0;
    if (audioWorkletNode) {
      audioWorkletNode.port.postMessage({ type: "reset" });
    }
    audioBtn.textContent = "Audio Enabled";
    setStatus("Audio enabled.");
    canvas.focus();
  } catch (err) {
    setStatus(`Audio init failed:\n${stringifyError(err)}`);
  }
});

document.addEventListener("keydown", (event) => {
  if (event.code === "F8") {
    event.preventDefault();
    captureFrameAndHud("hotkey-f8");
    return;
  }
  if (!emu || !romLoaded) {
    return;
  }
  const mapped = setKeyState(event.code, true);
  if (mapped) {
    event.preventDefault();
  }
}, true);

document.addEventListener("keyup", (event) => {
  if (!emu || !romLoaded) {
    return;
  }
  const mapped = setKeyState(event.code, false);
  if (mapped) {
    event.preventDefault();
  }
}, true);

window.addEventListener("blur", () => {
  if (!emu) {
    return;
  }
  clearControllerState();
  if (romLoaded) {
    updateHud();
  }
});

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState !== "visible") {
    clearControllerState();
  }
});

canvas.addEventListener("pointerdown", () => {
  canvas.focus();
});

function requestTick() {
  if (rafId !== null) {
    return;
  }
  rafId = window.requestAnimationFrame(tick);
}

function tick(nowMs) {
  rafId = null;
  if (!emu) {
    return;
  }

  if (lastTickTimestampMs === null) {
    lastTickTimestampMs = nowMs;
  }
  let deltaMs = nowMs - lastTickTimestampMs;
  lastTickTimestampMs = nowMs;
  if (!Number.isFinite(deltaMs) || deltaMs < 0) {
    deltaMs = 0;
  }
  if (deltaMs > MAX_DELTA_MS) {
    deltaMs = MAX_DELTA_MS;
  }

  wallFpsFrameCount += 1;
  if (nowMs - wallFpsWindowStartMs >= 1000) {
    const elapsed = nowMs - wallFpsWindowStartMs;
    wallFps = elapsed > 0 ? (wallFpsFrameCount * 1000) / elapsed : 0;
    wallFpsFrameCount = 0;
    wallFpsWindowStartMs = nowMs;
  }

  let steppedFrames = 0;
  if (running && romLoaded) {
    try {
      const targetFps = Math.max(1, emu.fps_milli() / 1000);
      const frameTimeMs = 1000 / targetFps;
      frameBudgetMs = Math.min(frameBudgetMs + deltaMs, frameTimeMs * MAX_CATCHUP_FRAMES);

      let framesSteppedThisTick = 0;
      while (frameBudgetMs >= frameTimeMs && framesSteppedThisTick < MAX_CATCHUP_FRAMES) {
        emu.step_frame();
        frameBudgetMs -= frameTimeMs;
        framesSteppedThisTick += 1;
        steppedFrames += 1;
        emuFramesStepped += 1;
        observeStateProgress();
        if (audioEnabled) {
          queueAudioChunk();
        }
      }

      // Keep lag bounded under load and avoid spiral-of-death behavior.
      if (framesSteppedThisTick === MAX_CATCHUP_FRAMES && frameBudgetMs > frameTimeMs) {
        frameBudgetMs = frameTimeMs;
      }
    } catch (err) {
      running = false;
      syncRunButton();
      setStatus(`Runtime error:\n${stringifyError(err)}`);
    }
  }

  if (romLoaded) {
    if (steppedFrames > 0) {
      renderFrame();
    }
    tickCount += 1;
    if (nowMs - emuFpsWindowStartMs >= 1000) {
      const elapsed = nowMs - emuFpsWindowStartMs;
      emuFps = elapsed > 0 ? (emuFpsFrameCount * 1000) / elapsed : 0;
      emuFpsFrameCount = 0;
      emuFpsWindowStartMs = nowMs;
    }
    if (tickCount % 30 === 0) {
      updateHud();
    }
  }

  if (running) {
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
  try {
    emu.set_speed(1000);
  } catch (_) {
    // Non-fatal in case runtime API changes.
  }
  clearControllerState();
  clearControllerLocks();
  romLoaded = true;
  running = false;
  tickCount = 0;
  resetTimingAndMetrics();
  syncRunButton();
  renderFrame();
  updateHud(message);
  canvas.focus();
}

async function loadRomBytesWithPersistence(bytes, romName, romSource, loadMessage) {
  loadRomBytes(bytes, loadMessage);
  const saveMessage = await persistRomForOfflineUse(bytes, romName, romSource);
  updateHud(`${loadMessage}\n${saveMessage}`);
}

async function persistRomForOfflineUse(bytes, romName, romSource) {
  if (!supportsRomStorage()) {
    hasSavedRom = false;
    syncForgetSavedRomButton();
    return "ROM storage unavailable in this browser.";
  }
  try {
    const saved = await saveRomForOfflineUse(bytes, romName, romSource);
    hasSavedRom = saved;
    syncForgetSavedRomButton();
    return saved
      ? "Saved ROM locally for next launch."
      : "ROM not saved locally.";
  } catch (err) {
    hasSavedRom = false;
    syncForgetSavedRomButton();
    return `ROM loaded, but local save failed: ${stringifyError(err)}`;
  }
}

async function tryRestoreSavedRom() {
  if (!supportsRomStorage()) {
    hasSavedRom = false;
    return false;
  }
  let savedRom = null;
  try {
    savedRom = await loadSavedRom();
  } catch (err) {
    hasSavedRom = false;
    setStatus(`WASM ready.\nSaved ROM lookup failed:\n${stringifyError(err)}`);
    return false;
  }
  if (!savedRom) {
    hasSavedRom = false;
    return false;
  }

  hasSavedRom = true;
  try {
    const savedAt = new Date(savedRom.savedAtMs).toLocaleString();
    loadRomBytes(savedRom.bytes, `Restored saved ROM: ${savedRom.name}`);
    updateHud(`Restored saved ROM: ${savedRom.name}\nSaved locally: ${savedAt}`);
    return true;
  } catch (err) {
    try {
      await clearSavedRom();
    } catch (_) {
      // Ignore secondary cleanup failure.
    }
    hasSavedRom = false;
    setStatus(
      "WASM ready.\nSaved ROM was invalid and has been removed.\n" +
      `Restore error: ${stringifyError(err)}`,
    );
    return false;
  }
}

function syncRunButton() {
  toggleRunBtn.textContent = running ? "Pause" : "Start";
  toggleRunBtn.setAttribute("aria-pressed", running ? "true" : "false");
}

function syncForgetSavedRomButton() {
  const storageSupported = supportsRomStorage();
  forgetSavedRomBtn.disabled = !(storageSupported && hasSavedRom);
  if (!storageSupported) {
    forgetSavedRomBtn.title = "IndexedDB not available in this browser.";
    return;
  }
  forgetSavedRomBtn.title = hasSavedRom
    ? "Delete locally stored ROM bytes from this device."
    : "No locally saved ROM found.";
}

function syncHoldRightButton() {
  const pressed = (lockedControllerBits & BUTTON_MASK.RIGHT) !== 0;
  holdRightBtn.textContent = pressed ? "Hold Right: On" : "Hold Right: Off";
  holdRightBtn.setAttribute("aria-pressed", pressed ? "true" : "false");
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
  lines.push(`wall_fps: ${wallFps.toFixed(1)}  emu_fps: ${emuFps.toFixed(1)}`);
  lines.push(`emu_frames: ${emuFramesStepped}`);
  lines.push(`unchanged_state_frames: ${unchangedStateFrames}`);
  lines.push(`pc_stall_frames: ${pcStallFrames}`);
  lines.push(`wait_loop_frames: ${waitLoopFrames}`);
  lines.push(`controller_bits: 0x${emu.controller_bits().toString(16).toUpperCase().padStart(2, "0")}`);
  lines.push(`input_bits(local): 0x${activeControllerBits.toString(16).toUpperCase().padStart(2, "0")}`);
  lines.push(`locked_bits: 0x${lockedControllerBits.toString(16).toUpperCase().padStart(2, "0")}`);
  lines.push(`auto_capture_attempts: ${autoCaptureAttempts}`);
  lines.push(`last_auto_capture: ${lastAutoCaptureReason}`);
  lines.push(`focused: ${document.hasFocus() ? "yes" : "no"}`);
  const activeEl = document.activeElement;
  const activeDesc = activeEl
    ? `${activeEl.tagName.toLowerCase()}${activeEl.id ? `#${activeEl.id}` : ""}`
    : "none";
  lines.push(`active_element: ${activeDesc}`);
  lines.push(`last_input: ${lastInputEvent} (#${inputEventCounter})`);
  lines.push(`audio: ${audioEnabled ? `enabled (${audioQueueMs.toFixed(1)}ms queue)` : "disabled"}`);
  lines.push("");
  lines.push("controls:");
  lines.push("arrows/WASD=dpad  z=a  x=b  enter=start  tab/c=select");
  lines.push("f8=capture");
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

function setKeyState(code, pressed) {
  const mask = KEY_TO_MASK.get(code);
  if (mask === undefined) {
    return false;
  }
  lastInputEvent = `${code}:${pressed ? "down" : "up"}`;
  inputEventCounter = inputEventCounter + 1;
  const nextBits = pressed
    ? (activeControllerBits | mask)
    : (activeControllerBits & ~mask);
  if (nextBits !== activeControllerBits) {
    activeControllerBits = nextBits;
    applyControllerState();
  }
  return true;
}

function clearControllerState() {
  if (!emu) {
    return;
  }
  activeControllerBits = 0;
  applyControllerState();
}

function clearControllerLocks() {
  lockedControllerBits = 0;
  syncHoldRightButton();
  applyControllerState();
}

function applyControllerState() {
  if (!emu) {
    return;
  }
  try {
    emu.set_controller_state(activeControllerBits | lockedControllerBits);
  } catch (err) {
    setStatus(`Input dispatch failed:\n${stringifyError(err)}`);
    running = false;
    syncRunButton();
  }
}

async function ensureAudioPipeline(sampleRate) {
  if (!audioContext) {
    audioContext = new AudioContext({ sampleRate, latencyHint: "interactive" });
  }
  if (!audioContext.audioWorklet) {
    throw new Error("AudioWorklet is not supported by this browser.");
  }
  if (!audioWorkletReady) {
    await audioContext.audioWorklet.addModule("./audio-worklet.js");
    audioWorkletReady = true;
  }
  if (!audioWorkletNode) {
    audioWorkletNode = new AudioWorkletNode(audioContext, "nes-audio-processor", {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [1],
    });
    audioWorkletNode.port.onmessage = (event) => {
      const msg = event.data;
      if (!msg || typeof msg !== "object") {
        return;
      }
      if (msg.type === "queue_samples" && Number.isFinite(msg.queuedSamples)) {
        audioQueuedSamples = Math.max(0, Math.floor(msg.queuedSamples));
        audioQueueMs = (audioQueuedSamples * 1000) / audioContext.sampleRate;
      }
    };
    audioWorkletNode.connect(audioContext.destination);
  }
  const maxSamples = Math.max(1, Math.floor((audioContext.sampleRate * AUDIO_MAX_QUEUE_MS) / 1000));
  const targetSamples = Math.max(1, Math.floor((audioContext.sampleRate * AUDIO_TARGET_QUEUE_MS) / 1000));
  audioWorkletNode.port.postMessage({
    type: "configure",
    maxQueueSamples: maxSamples,
    targetQueueSamples: targetSamples,
  });
}

function queueAudioChunk() {
  if (!emu || !audioContext || !audioWorkletNode || audioContext.state !== "running") {
    return;
  }
  const chunk = emu.audio_chunk_i16();
  if (chunk.length === 0) {
    return;
  }

  const samples = new Float32Array(chunk.length);
  for (let idx = 0; idx < chunk.length; idx += 1) {
    samples[idx] = Math.max(-1, Math.min(1, chunk[idx] / 32768));
  }
  audioWorkletNode.port.postMessage({ type: "push", samples }, [samples.buffer]);
  audioQueuedSamples += chunk.length;
  audioQueueMs = (audioQueuedSamples * 1000) / audioContext.sampleRate;
}

function resetTimingAndMetrics() {
  const now = performance.now();
  lastTickTimestampMs = null;
  frameBudgetMs = 0;
  wallFps = 0;
  wallFpsFrameCount = 0;
  wallFpsWindowStartMs = now;
  emuFramesStepped = 0;
  emuFps = 0;
  emuFpsFrameCount = 0;
  emuFpsWindowStartMs = now;
  lastStateHash = 0;
  unchangedStateFrames = 0;
  activeControllerBits = 0;
  lastInputEvent = "none";
  inputEventCounter = 0;
  lastPc = 0;
  pcStallFrames = 0;
  autoCapturedStall = false;
  waitLoopFrames = 0;
  autoCaptureAttempts = 0;
  lastAutoCaptureReason = "none";
  lockedControllerBits = 0;
  audioQueuedSamples = 0;
  audioQueueMs = 0;
  if (audioWorkletNode) {
    audioWorkletNode.port.postMessage({ type: "reset" });
  }
  syncHoldRightButton();
}

function observeStateProgress() {
  if (!emu) {
    return;
  }
  emuFpsFrameCount += 1;
  const pc = emu.cpu_pc();
  if (pc === lastPc) {
    pcStallFrames += 1;
  } else {
    pcStallFrames = 0;
    lastPc = pc;
  }
  if (pc >= SMB_WAIT_PC_MIN && pc <= SMB_WAIT_PC_MAX) {
    waitLoopFrames += 1;
  } else {
    waitLoopFrames = 0;
  }
  if (
    running &&
    !autoCapturedStall &&
    activeControllerBits !== 0 &&
    pcStallFrames >= AUTO_CAPTURE_PC_STALL_FRAMES
  ) {
    autoCapturedStall = true;
    captureFrameAndHud(`auto-pcstall-${pc.toString(16).toUpperCase().padStart(4, "0")}`);
  }
  if (
    running &&
    !autoCapturedStall &&
    waitLoopFrames >= AUTO_CAPTURE_WAIT_LOOP_FRAMES
  ) {
    autoCapturedStall = true;
    captureFrameAndHud(`auto-waitloop-${pc.toString(16).toUpperCase().padStart(4, "0")}`);
  }
  const stateHash = emu.state_hash();
  if (stateHash === lastStateHash) {
    unchangedStateFrames += 1;
  } else {
    unchangedStateFrames = 0;
    lastStateHash = stateHash;
  }
}

function captureFrameAndHud(reason) {
  if (!romLoaded) {
    return;
  }
  lastAutoCaptureReason = reason;
  autoCaptureAttempts += 1;
  updateHud();
  const stamp = new Date().toISOString().replaceAll(":", "-");
  const frameName = `nes-web-${reason}-${stamp}.png`;
  const hudName = `nes-web-${reason}-${stamp}.txt`;
  const hudDump = statusEl.textContent ?? "";
  try {
    localStorage.setItem("nes_web_last_capture_reason", reason);
    localStorage.setItem("nes_web_last_capture_hud", hudDump);
  } catch (_) {
    // Ignore storage availability issues.
  }
  console.info(`[capture] ${reason}`);
  canvas.toBlob((blob) => {
    if (!blob) {
      setStatus("Capture failed: canvas blob unavailable.");
      return;
    }
    downloadBlob(blob, frameName);
  }, "image/png");
  downloadBlob(new Blob([hudDump], { type: "text/plain" }), hudName);
}

function downloadBlob(blob, fileName) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.style.display = "none";
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
