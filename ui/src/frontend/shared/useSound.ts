import { useCallback, useRef } from 'react';

let audioCtx: AudioContext | null = null;

function getAudioCtx(): AudioContext | null {
  if (!audioCtx) {
    try {
      audioCtx = new AudioContext();
    } catch {
      return null;
    }
  }
  if (audioCtx.state === 'suspended') {
    audioCtx.resume();
  }
  return audioCtx;
}

export function useSound() {
  const enabledRef = useRef(true);

  const setEnabled = useCallback((v: boolean) => {
    enabledRef.current = v;
  }, []);

  const playBeep = useCallback(() => {
    if (!enabledRef.current) return;
    const ctx = getAudioCtx();
    if (!ctx) return;
    try {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 880;
      osc.type = 'sine';
      gain.gain.value = 0.25;
      osc.start();
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.12);
      osc.stop(ctx.currentTime + 0.12);
    } catch { /* ignore */ }
  }, []);

  const playError = useCallback(() => {
    if (!enabledRef.current) return;
    const ctx = getAudioCtx();
    if (!ctx) return;
    try {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 180;
      osc.type = 'sawtooth';
      gain.gain.value = 0.2;
      osc.start();
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.35);
      osc.stop(ctx.currentTime + 0.35);
    } catch { /* ignore */ }
  }, []);

  const playSuccess = useCallback(() => {
    if (!enabledRef.current) return;
    const ctx = getAudioCtx();
    if (!ctx) return;
    try {
      const notes = [523, 659, 784];
      notes.forEach((freq, i) => {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.frequency.value = freq;
        osc.type = 'sine';
        gain.gain.setValueAtTime(0.25, ctx.currentTime + i * 0.1);
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + i * 0.1 + 0.25);
        osc.start(ctx.currentTime + i * 0.1);
        osc.stop(ctx.currentTime + i * 0.1 + 0.3);
      });
    } catch { /* ignore */ }
  }, []);

  const playAlert = useCallback(() => {
    if (!enabledRef.current) return;
    const ctx = getAudioCtx();
    if (!ctx) return;
    try {
      // Three ascending pulses: 523Hz → 659Hz → 784Hz (C5 → E5 → G5)
      [523, 659, 784].forEach((freq, i) => {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.frequency.value = freq;
        osc.type = 'square';
        gain.gain.setValueAtTime(0.18, ctx.currentTime + i * 0.18);
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + i * 0.18 + 0.15);
        osc.start(ctx.currentTime + i * 0.18);
        osc.stop(ctx.currentTime + i * 0.18 + 0.2);
      });
    } catch { /* ignore */ }
  }, []);

  return { playBeep, playError, playSuccess, playAlert, setSoundEnabled: setEnabled };
}
