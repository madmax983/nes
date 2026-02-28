class NesAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.queue = [];
    this.readOffset = 0;
    this.queuedSamples = 0;
    this.maxQueueSamples = Math.floor(sampleRate * 0.035);
    this.targetQueueSamples = Math.floor(sampleRate * 0.016);
    this.reportEveryFrames = 8;
    this.frameCounter = 0;

    this.port.onmessage = (event) => {
      const msg = event.data;
      if (!msg || typeof msg !== "object") {
        return;
      }
      if (msg.type === "configure") {
        if (Number.isFinite(msg.maxQueueSamples) && msg.maxQueueSamples > 0) {
          this.maxQueueSamples = Math.floor(msg.maxQueueSamples);
        }
        if (Number.isFinite(msg.targetQueueSamples) && msg.targetQueueSamples > 0) {
          this.targetQueueSamples = Math.floor(msg.targetQueueSamples);
        }
        if (this.targetQueueSamples > this.maxQueueSamples) {
          this.targetQueueSamples = this.maxQueueSamples;
        }
        this.trimQueue();
        return;
      }
      if (msg.type === "reset") {
        this.queue = [];
        this.readOffset = 0;
        this.queuedSamples = 0;
        this.reportQueue();
        return;
      }
      if (msg.type === "push") {
        if (!msg.samples) {
          return;
        }
        let samples = msg.samples;
        if (!(samples instanceof Float32Array)) {
          samples = new Float32Array(samples);
        }
        if (samples.length === 0) {
          return;
        }
        this.queue.push(samples);
        this.queuedSamples += samples.length;
        this.trimQueue();
      }
    };
  }

  trimQueue() {
    if (this.queuedSamples <= this.maxQueueSamples) {
      return;
    }
    while (this.queuedSamples > this.targetQueueSamples && this.queue.length > 0) {
      const head = this.queue[0];
      const available = head.length - this.readOffset;
      const drop = Math.min(available, this.queuedSamples - this.targetQueueSamples);
      this.readOffset += drop;
      this.queuedSamples -= drop;
      if (this.readOffset >= head.length) {
        this.queue.shift();
        this.readOffset = 0;
      }
    }
  }

  reportQueue() {
    this.port.postMessage({
      type: "queue_samples",
      queuedSamples: this.queuedSamples,
    });
  }

  process(_inputs, outputs) {
    const output = outputs[0][0];
    output.fill(0);

    let outIndex = 0;
    while (outIndex < output.length && this.queue.length > 0) {
      const head = this.queue[0];
      const available = head.length - this.readOffset;
      const copyCount = Math.min(available, output.length - outIndex);
      output.set(head.subarray(this.readOffset, this.readOffset + copyCount), outIndex);
      outIndex += copyCount;
      this.readOffset += copyCount;
      this.queuedSamples -= copyCount;
      if (this.readOffset >= head.length) {
        this.queue.shift();
        this.readOffset = 0;
      }
    }

    this.frameCounter += 1;
    if (this.frameCounter >= this.reportEveryFrames) {
      this.frameCounter = 0;
      this.reportQueue();
    }
    return true;
  }
}

registerProcessor("nes-audio-processor", NesAudioProcessor);
