// AudioWorklet processor for capturing audio samples
class AudioCaptureProcessor extends AudioWorkletProcessor {
  process(inputs) {
    const ch = inputs[0]?.[0];
    if (ch?.length > 0) {
      this.port.postMessage(new Float32Array(ch));
    }
    return true;
  }
}

registerProcessor('audio-capture', AudioCaptureProcessor);
