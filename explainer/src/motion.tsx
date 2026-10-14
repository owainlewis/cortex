import React from 'react';
import {Easing, interpolate, useCurrentFrame} from 'remotion';
import {fonts, useStyle} from './theme';
import type {Word} from './types';

export const clamp = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

// One motion language everywhere: a fast expo-out for entrances and a short
// ease-in for exits. No bounce.
export const easeOut = Easing.bezier(0.16, 1, 0.3, 1);
export const easeIn = Easing.bezier(0.7, 0, 0.84, 0);

export const progress = (frame: number, start: number, duration = 16, easing = easeOut) =>
  interpolate(frame, [start, start + duration], [0, 1], {...clamp, easing});

export const useProgress = (start: number, duration = 16, easing = easeOut) =>
  progress(useCurrentFrame(), start, duration, easing);

// Fade and lift for small elements that do not need a mask.
export const lift = (p: number, distance = 24): React.CSSProperties => ({
  opacity: p,
  transform: `translateY(${(1 - p) * distance}px)`,
});

const normalize = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

// For each display token, the frame at which the narrator says it, so titles
// assemble in time with the voice. Tokens the narrator never says fall back to
// a short stagger after the previous token.
export const syncDelays = (tokens: string[], words: Word[] | undefined, start = 4, stagger = 3): number[] => {
  const out: number[] = [];
  let pointer = 0;
  let last = start - stagger;
  for (const token of tokens) {
    const key = normalize(token);
    let hit = -1;
    if (words && key) {
      for (let i = pointer; i < Math.min(words.length, pointer + 6); i++) {
        if (normalize(words[i].text) === key) {
          hit = i;
          break;
        }
      }
    }
    let delay = last + stagger;
    if (hit >= 0) {
      delay = Math.max(last + 1, words![hit].start - 3);
      pointer = hit + 1;
    }
    out.push(delay);
    last = delay;
  }
  return out;
};

type Segment = {text: string; mark: 'none' | 'accent' | 'highlight'};

// `*word*` sets in serif italic, `==words==` sits on a flat accent block.
export const parseRich = (text: string): Segment[] => {
  const out: Segment[] = [];
  const re = /(\*[^*]+\*|==[^=]+==)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push({text: text.slice(last, m.index), mark: 'none'});
    const raw = m[0];
    if (raw.startsWith('==')) out.push({text: raw.slice(2, -2), mark: 'highlight'});
    else out.push({text: raw.slice(1, -1), mark: 'accent'});
    last = m.index + raw.length;
  }
  if (last < text.length) out.push({text: text.slice(last), mark: 'none'});
  return out;
};

// A word rising from behind its own baseline mask.
const Masked: React.FC<{p: number; children: React.ReactNode; style?: React.CSSProperties}> = ({p, children, style}) => (
  <span
    style={{
      display: 'inline-block',
      overflow: 'hidden',
      verticalAlign: 'top',
      padding: '0.06em 0.04em 0.16em',
      margin: '-0.06em -0.04em -0.16em',
    }}
  >
    <span style={{display: 'inline-block', whiteSpace: 'pre', transform: `translateY(${(1 - p) * 115}%)`, ...style}}>{children}</span>
  </span>
);

export const RichText: React.FC<{
  text: string;
  size: number;
  weight?: number;
  color?: string;
  align?: 'left' | 'center';
  lineHeight?: number;
  letterSpacing?: string;
  words?: Word[];
  start?: number;
  stagger?: number;
  sync?: boolean;
}> = ({text, size, weight = 650, color, align = 'left', lineHeight = 1.02, letterSpacing = '-0.04em', words, start = 4, stagger = 2, sync = false}) => {
  const {theme} = useStyle();
  const frame = useCurrentFrame();
  const segments = parseRich(text).map((seg) => ({...seg, words: seg.text.split(/\s+/).filter(Boolean)}));
  const delays = syncDelays(segments.flatMap((s) => s.words), sync ? words : undefined, start, stagger);

  let k = 0;
  const parts: React.ReactNode[] = [];
  segments.forEach((seg, s) => {
    if (!seg.words.length) return;
    const first = k;
    const nodes = seg.words.map((w) => {
      const i = k++;
      const p = progress(frame, delays[i], 18);
      const style: React.CSSProperties =
        seg.mark === 'accent'
          ? {fontFamily: fonts.serif, fontStyle: 'italic', fontWeight: 400, letterSpacing: '-0.01em', color: theme.accent}
          : seg.mark === 'highlight'
            ? {color: theme.accentInk}
            : {};
      return (
        <Masked key={i} p={p} style={style}>
          {w}
        </Masked>
      );
    });
    const spaced = nodes.flatMap((n, i) => (i ? [' ', n] : [n]));
    if (seg.mark === 'highlight') {
      // The block grows with its words rather than ahead of them.
      const wipe = progress(frame, delays[first] - 2, Math.max(14, delays[k - 1] - delays[first] + 12));
      parts.push(
        <span key={`g${s}`} style={{position: 'relative', display: 'inline-block', whiteSpace: 'nowrap', isolation: 'isolate'}}>
          <span
            style={{
              position: 'absolute',
              left: '-0.1em',
              right: '-0.1em',
              top: '0.06em',
              bottom: '-0.04em',
              background: theme.accent,
              transform: `scaleX(${wipe})`,
              transformOrigin: 'left',
              zIndex: -1,
            }}
          />
          {spaced}
        </span>,
      );
    } else if (seg.mark === 'accent') {
      parts.push(
        <span key={`g${s}`} style={{whiteSpace: 'nowrap'}}>
          {spaced}
        </span>,
      );
    } else {
      parts.push(...nodes);
    }
  });

  return (
    <div style={{fontSize: size, fontWeight: weight, lineHeight, letterSpacing, color: color ?? theme.text, textAlign: align, textWrap: 'balance'}}>
      {parts.flatMap((n, i) => (i ? [' ', n] : [n]))}
    </div>
  );
};

// Small mono label used above headings and on diagram parts.
export const Label: React.FC<{text?: string; delay?: number; color?: string; style?: React.CSSProperties}> = ({text, delay = 0, color, style}) => {
  const {theme, layout} = useStyle();
  const p = useProgress(delay, 14);
  if (!text) return null;
  return (
    <div
      style={{
        fontFamily: fonts.mono,
        fontSize: layout.small,
        fontWeight: 500,
        letterSpacing: '0.06em',
        textTransform: 'uppercase',
        color: color ?? theme.muted,
        ...lift(p, 12),
        ...style,
      }}
    >
      {text}
    </div>
  );
};

// A hairline that draws from the left.
export const Rule: React.FC<{delay?: number; color?: string; thickness?: number}> = ({delay = 0, color, thickness = 2}) => {
  const {theme} = useStyle();
  const p = useProgress(delay, 22);
  return <div style={{width: '100%', height: thickness, background: color ?? theme.line, transform: `scaleX(${p})`, transformOrigin: 'left'}} />;
};

// Index of the most recently revealed cue, or -1 before the first.
export const activeCue = (cues: number[], frame: number) => {
  let active = -1;
  cues.forEach((c, i) => {
    if (frame >= c) active = i;
  });
  return active;
};

export const formatNumber = (value: number, decimals = 0) =>
  value.toLocaleString('en-US', {minimumFractionDigits: decimals, maximumFractionDigits: decimals});

// Drawn rather than typed: the bundled latin font subsets have no arrows.
export const ArrowIcon: React.FC<{size: number; color: string}> = ({size, color}) => (
  <svg width={size} height={size} viewBox="0 0 24 24" style={{flex: 'none'}}>
    <path d="M3 12h17M14 6l6 6-6 6" fill="none" stroke={color} strokeWidth="2" strokeLinecap="square" />
  </svg>
);

export const MarkIcon: React.FC<{good: boolean; size: number; color: string}> = ({good, size, color}) => (
  <svg width={size} height={size} viewBox="0 0 24 24" style={{flex: 'none'}}>
    {good ? (
      <path d="M4 12.5l5 5L20 6.5" fill="none" stroke={color} strokeWidth="2.4" strokeLinecap="square" />
    ) : (
      <path d="M5 5l14 14M19 5L5 19" fill="none" stroke={color} strokeWidth="2.4" strokeLinecap="square" />
    )}
  </svg>
);
