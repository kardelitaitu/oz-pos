/** Named interaction events that can trigger sound and vibration feedback. */
export type InteractionName =
  | 'add-to-cart'
  | 'qty-change'
  | 'remove-item'
  | 'undo-cart'
  | 'pay'
  | 'open-bill';

interface InteractionConfig {
  sound: string;
  vibrate: boolean;
}

const INTERACTIONS: Record<InteractionName, InteractionConfig> = {
  'add-to-cart': { sound: 'click.mp3', vibrate: false },
  'qty-change':  { sound: 'click.mp3', vibrate: false },
  'remove-item': { sound: 'click.mp3', vibrate: false },
  'undo-cart':   { sound: 'click.mp3', vibrate: false },
  'pay':         { sound: 'click.mp3', vibrate: false },
  'open-bill':   { sound: 'click.mp3', vibrate: false },
};

const audioCache = new Map<string, HTMLAudioElement>();

function getAudio(filename: string): HTMLAudioElement | null {
  const cached = audioCache.get(filename);
  if (cached) return cached;
  try {
    const url = new URL(`../assets/sounds/${filename}`, import.meta.url).href;
    const audio = new Audio(url);
    audio.volume = 0.25;
    audioCache.set(filename, audio);
    return audio;
  } catch {
    return null;
  }
}

/** Play the configured sound and (optionally) vibrate for the given interaction. */
export function triggerInteraction(name: InteractionName): void {
  const config = INTERACTIONS[name];
  if (!config) return;

  const audio = getAudio(config.sound);
  if (audio) {
    audio.currentTime = 0;
    audio.play().catch(() => {});
  }

  if (config.vibrate && navigator.vibrate) {
    navigator.vibrate(15);
  }
}
