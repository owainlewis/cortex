import React from 'react';
import {interpolate, interpolateColors, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {useStyle} from './theme';
import type {Word} from './types';

export const springs = {
  smooth: {damping: 200, mass: 1, stiffness: 120},
  snappy: {damping: 20, stiffness: 240, mass: 0.7},
  bouncy: {damping: 12, stiffness: 170, mass: 0.8},
};

export const useSpringAt = (delay: number, config = springs.snappy) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  return spring({frame: frame - delay, fps, config});
};

// Opacity, lift and focus pull for an element entering with progress `p`.
export const riseStyle = (p: number, distance = 48, blur = 12): React.CSSProperties => ({
  opacity: interpolate(p, [0, 0.6], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
  transform: `translateY(${(1 - p) * distance}px)`,
  filter: p < 0.999 ? `blur(${(1 - Math.min(p, 1)) * blur}px)` : undefined,
});

export const clamp = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

const normalize = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

// For each display token, the frame at which the narrator says it, so titles
// assemble in time with the voice. Tokens the narrator never says fall back to
// a short stagger after the previous token.
export const syncDelays = (
  tokens: string[],
  words: Word[] | undefined,
  start = 4,
  stagger = 3,
): number[] => {
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

// `*word*` renders in the accent colour, `==words==` gets a highlighter swipe.
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

export const plainText = (text: string) => parseRich(text).map((s) => s.text).join('');

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
  maxWidth?: number;
}> = ({
  text,
  size,
  weight = 800,
  color,
  align = 'left',
  lineHeight = 1.06,
  letterSpacing = '-0.03em',
  words,
  start = 4,
  stagger = 3,
  sync = false,
  maxWidth,
}) => {
  const {theme} = useStyle();
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const segments = parseRich(text).map((seg) => ({...seg, words: seg.text.split(/\s+/).filter(Boolean)}));
  const flat = segments.flatMap((seg) => seg.words);
  const delays = syncDelays(flat, sync ? words : undefined, start, stagger);

  let k = 0;
  const word = (w: string, wordColor?: string) => {
    const i = k++;
    const p = spring({frame: frame - delays[i], fps, config: springs.snappy});
    return (
      <span key={i} style={{display: 'inline-block', whiteSpace: 'pre', color: wordColor, ...riseStyle(p, size * 0.45, 10)}}>
        {w}
      </span>
    );
  };
  const spaced = (nodes: React.ReactNode[]) => nodes.flatMap((n, i) => (i ? [' ', n] : [n]));

  const parts: React.ReactNode[] = [];
  segments.forEach((seg, s) => {
    if (!seg.words.length) return;
    if (seg.mark === 'none') {
      seg.words.forEach((w) => parts.push(word(w)));
      return;
    }
    // Marked phrases stay on one line and animate as a unit.
    const first = k;
    if (seg.mark === 'accent') {
      parts.push(
        <span key={`g${s}`} style={{display: 'inline-block', whiteSpace: 'nowrap'}}>
          {spaced(seg.words.map((w) => word(w, theme.accent)))}
        </span>,
      );
      return;
    }
    const swipe = spring({frame: frame - delays[first] - 3, fps, config: springs.smooth});
    const ink = interpolateColors(swipe, [0.2, 0.6], [color ?? theme.text, theme.accentInk]);
    parts.push(
      <span key={`g${s}`} style={{display: 'inline-block', whiteSpace: 'nowrap', position: 'relative', isolation: 'isolate'}}>
        <span
          style={{
            position: 'absolute',
            left: '-0.14em',
            right: '-0.14em',
            top: '0.04em',
            bottom: '0.0em',
            background: theme.accent,
            borderRadius: '0.14em',
            transform: `scaleX(${swipe}) skewX(-6deg)`,
            transformOrigin: 'left center',
            zIndex: -1,
            boxShadow: theme.dark ? `0 0 ${50 * swipe}px ${withAlpha(theme.accent, 0.35)}` : undefined,
          }}
        />
        {spaced(seg.words.map((w) => word(w, ink)))}
      </span>,
    );
  });

  return (
    <div
      style={{
        fontSize: size,
        fontWeight: weight,
        lineHeight,
        letterSpacing,
        color: color ?? theme.text,
        textAlign: align,
        maxWidth,
        textWrap: 'balance',
      }}
    >
      {spaced(parts)}
    </div>
  );
};

export const Kicker: React.FC<{text?: string; delay?: number}> = ({text, delay = 0}) => {
  const {theme} = useStyle();
  const p = useSpringAt(delay, springs.smooth);
  if (!text) return null;
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        fontSize: 30,
        fontWeight: 700,
        letterSpacing: '0.16em',
        textTransform: 'uppercase',
        color: theme.muted,
        ...riseStyle(p, 20, 6),
      }}
    >
      <span
        style={{
          width: 14,
          height: 14,
          borderRadius: 7,
          background: theme.accent,
          boxShadow: `0 0 24px ${theme.accent}`,
        }}
      />
      {text}
    </div>
  );
};

export const Panel: React.FC<{
  children: React.ReactNode;
  style?: React.CSSProperties;
  glow?: number;
}> = ({children, style, glow = 0}) => {
  const {theme} = useStyle();
  return (
    <div
      style={{
        background: theme.panel,
        border: `2px solid ${glow > 0 ? mix(theme.panelBorder, theme.accent, glow) : theme.panelBorder}`,
        borderRadius: 28,
        boxShadow: theme.dark
          ? `0 30px 80px rgba(0,0,0,0.45), inset 0 1px 0 rgba(255,255,255,0.06)${
              glow > 0 ? `, 0 0 ${60 * glow}px ${withAlpha(theme.accent, 0.35 * glow)}` : ''
            }`
          : `0 20px 50px rgba(40,30,10,0.12)${glow > 0 ? `, 0 0 ${40 * glow}px ${withAlpha(theme.accent, 0.3 * glow)}` : ''}`,
        ...style,
      }}
    >
      {children}
    </div>
  );
};

export const withAlpha = (hex: string, alpha: number) => {
  if (!hex.startsWith('#')) return hex;
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
};

// Blend toward `b` by `t`. Only used for borders so a cheap rgba overlay is fine.
const mix = (a: string, b: string, t: number) => (t > 0.5 ? withAlpha(b, 0.4 + 0.5 * t) : a);

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
  <svg width={size} height={size} viewBox="0 0 24 24" style={{flex: 'none', transform: 'translateY(0.12em)'}}>
    <path d="M4 12h15M13 6l6 6-6 6" fill="none" stroke={color} strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

export const ChevronIcon: React.FC<{size: number; color: string}> = ({size, color}) => (
  <svg width={size} height={size} viewBox="0 0 24 24" style={{flex: 'none', alignSelf: 'center'}}>
    <path d="M8 5l7 7-7 7" fill="none" stroke={color} strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);
