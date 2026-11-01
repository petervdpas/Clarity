// The sound part (DESIGN 5.22) in the renderer: what code tells a sound,
// played with the Web Audio API. Files come from the resources next to the
// renderer (or carried in the page, on C and C++); tones and melodies are
// oscillators. A melody's notes read as conformance/melody.json says.

/** A note of a melody: its frequency (null for a rest) and how long it lasts. */
export type Note = { hertz: number | null; seconds: number };

const SEMITONES: Record<string, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
const NOTE = /^(?:([A-G])([#b]?)([0-8]?)|(-))(?:\*([1-9][0-9]*)|\/([1-9][0-9]*))?$/;

/** The notes of `notes` at `tempo` beats a minute, and the words that are not notes. */
export function melody(notes: string, tempo: number): { plays: Note[]; skips: string[] } {
  const beat = 60 / Math.max(1, tempo);
  const plays: Note[] = [];
  const skips: string[] = [];
  for (const word of notes.split(/\s+/).filter((w) => w !== "")) {
    const m = NOTE.exec(word);
    if (!m) {
      skips.push(word);
      continue;
    }
    const beats = m[5] ? Number(m[5]) : m[6] ? 1 / Number(m[6]) : 1;
    if (m[4]) {
      plays.push({ hertz: null, seconds: beats * beat });
      continue;
    }
    const shift = m[2] === "#" ? 1 : m[2] === "b" ? -1 : 0;
    const octave = m[3] ? Number(m[3]) : 4;
    const midi = 12 * (octave + 1) + SEMITONES[m[1]] + shift;
    plays.push({ hertz: 440 * 2 ** ((midi - 69) / 12), seconds: beats * beat });
  }
  return { plays, skips };
}

/** How a sound part is set: its file, its volume (0 to 100) and whether it repeats. */
export type SoundSettings = { source: string | null; volume: number | null; repeat: boolean };

/** How long a tone takes to swell and fade, so it starts and ends without a click. */
const EDGE = 0.01;

/** One sound part of a window: what it plays, and its volume. */
export class SoundPart {
  #context: AudioContext | null = null;
  #volume: number;
  #settings: SoundSettings;
  #url: (name: string) => string;
  #playing: AudioScheduledSourceNode[] = [];
  #files = new Map<string, Promise<AudioBuffer>>();

  constructor(settings: SoundSettings, url: (name: string) => string) {
    this.#settings = settings;
    this.#volume = settings.volume ?? 100;
    this.#url = url;
  }

  #audio(): AudioContext {
    this.#context ??= new AudioContext();
    if (this.#context.state === "suspended") void this.#context.resume();
    return this.#context;
  }

  #gain(context: AudioContext): GainNode {
    const gain = context.createGain();
    gain.gain.value = Math.max(0, Math.min(100, this.#volume)) / 100;
    gain.connect(context.destination);
    return gain;
  }

  #started(node: AudioScheduledSourceNode) {
    this.#playing.push(node);
    node.onended = () => (this.#playing = this.#playing.filter((n) => n !== node));
  }

  /** Does what code told it: `play`, `stop`, `tone`, `melody` or `volume`. */
  hear(hear: string, values: unknown[]) {
    switch (hear) {
      case "play":
        return void this.play();
      case "stop":
        return this.stop();
      case "tone":
        return this.tone(Number(values[0]), Number(values[1]));
      case "melody":
        return this.melody(String(values[0] ?? ""), Number(values[1]));
      case "volume":
        this.#volume = Number(values[0]);
        return;
    }
  }

  /** Plays its source from the start, over and over when it repeats. */
  async play() {
    const name = this.#settings.source;
    if (!name) return;
    const context = this.#audio();
    let file = this.#files.get(name);
    if (!file) {
      file = fetch(this.#url(name))
        .then((r) => r.arrayBuffer())
        .then((bytes) => context.decodeAudioData(bytes));
      this.#files.set(name, file);
    }
    const buffer = await file.catch(() => null);
    if (!buffer) return;
    const source = context.createBufferSource();
    source.buffer = buffer;
    source.loop = this.#settings.repeat;
    source.connect(this.#gain(context));
    this.#started(source);
    source.start();
  }

  /** Stops everything it plays. */
  stop() {
    for (const node of this.#playing) {
      try {
        node.stop();
      } catch {
        // Already ended.
      }
    }
    this.#playing = [];
  }

  /** A tone of `hertz` for `seconds`, starting `at` seconds from now. */
  tone(hertz: number, seconds: number, at = 0) {
    if (!(hertz > 0) || !(seconds > 0)) return;
    const context = this.#audio();
    const start = context.currentTime + at;
    const gain = this.#gain(context);
    const level = gain.gain.value;
    const edge = Math.min(EDGE, seconds / 4);
    gain.gain.setValueAtTime(0, start);
    gain.gain.linearRampToValueAtTime(level, start + edge);
    gain.gain.setValueAtTime(level, start + seconds - edge);
    gain.gain.linearRampToValueAtTime(0, start + seconds);
    const oscillator = context.createOscillator();
    oscillator.frequency.value = hertz;
    oscillator.connect(gain);
    this.#started(oscillator);
    oscillator.start(start);
    oscillator.stop(start + seconds);
  }

  /** Plays the notes one after another; words that are not notes are skipped. */
  melody(notes: string, tempo: number) {
    let at = 0;
    for (const note of melody(notes, tempo).plays) {
      if (note.hertz !== null) this.tone(note.hertz, note.seconds, at);
      at += note.seconds;
    }
  }

  /** Stops it and lets the audio go, when its window closes. */
  close() {
    this.stop();
    void this.#context?.close();
    this.#context = null;
  }
}
