// The timeline is produced by the Python pipeline (pipeline/timeline.py) and
// passed to the composition as input props. All times are in frames.

export type Word = {
  text: string;
  start: number;
  end: number;
};

export type ThemeName = 'ink' | 'paper' | 'blueprint';

export type Format = 'vertical' | 'square' | 'landscape';

export type SceneTiming = {
  type: string;
  // Scene content exactly as written in the spec.
  data: Record<string, any>;
  // Absolute start frame and length of the narration window.
  from: number;
  duration: number;
  // Frames, relative to `from`, at which the scene reveals its next element.
  cues: number[];
  // Narrated words, relative to `from`.
  words: Word[];
  // Scene specific timing computed by the pipeline (e.g. typing windows).
  extra: Record<string, number>;
  captions: boolean;
};

export type Timeline = {
  title: string;
  fps: number;
  width: number;
  height: number;
  format: Format;
  theme: ThemeName;
  durationInFrames: number;
  audio: string | null;
  handle: string | null;
  scenes: SceneTiming[];
  // Narrated words, absolute frames. Drives the karaoke captions.
  words: Word[];
};

export type SceneProps = {
  scene: SceneTiming;
  index: number;
};
