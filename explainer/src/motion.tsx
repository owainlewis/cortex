import {icons, type LucideIcon} from 'lucide-react';
import React from 'react';
import {Easing, interpolate, useCurrentFrame} from 'remotion';
import {fonts, useStyle} from './theme';
import type {Word} from './types';

export const clamp = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

// Soft, confident motion: a long ease-out for entrances, a short ease-in for
// exits. Nothing bounces.
export const easeOut = Easing.bezier(0.22, 1, 0.36, 1);
export const easeIn = Easing.bezier(0.55, 0, 1, 0.45);

export const progress = (frame: number, start: number, duration = 18, easing = easeOut) =>
  interpolate(frame, [start, start + duration], [0, 1], {...clamp, easing});

export const useProgress = (start: number, duration = 18, easing = easeOut) =>
  progress(useCurrentFrame(), start, duration, easing);

// Fade, rise and settle from a hair below full size.
export const enter = (p: number, distance = 18): React.CSSProperties => ({
  opacity: p,
  transform: `translateY(${(1 - p) * distance}px) scale(${0.97 + 0.03 * p})`,
});

const normalize = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

// For each display token, the frame at which the narrator says it, so titles
// assemble in time with the voice. Unsaid tokens follow a short stagger.
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

type Segment = {text: string; mark: 'none' | 'italic' | 'accent'};

// `*word*` sets in italic, `==words==` in the accent colour.
export const parseRich = (text: string): Segment[] => {
  const out: Segment[] = [];
  const re = /(\*[^*]+\*|==[^=]+==)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push({text: text.slice(last, m.index), mark: 'none'});
    const raw = m[0];
    out.push(raw.startsWith('==') ? {text: raw.slice(2, -2), mark: 'accent'} : {text: raw.slice(1, -1), mark: 'italic'});
    last = m.index + raw.length;
  }
  if (last < text.length) out.push({text: text.slice(last), mark: 'none'});
  return out;
};

export const plain = (text: string) => parseRich(text).map((s) => s.text).join('');

// Text that settles in word by word. `serif` is the headline voice.
export const RichText: React.FC<{
  text: string;
  size: number;
  serif?: boolean;
  weight?: number;
  color?: string;
  align?: 'left' | 'center';
  lineHeight?: number;
  words?: Word[];
  start?: number;
  stagger?: number;
  sync?: boolean;
}> = ({text, size, serif = false, weight, color, align = 'left', lineHeight, words, start = 4, stagger = 2, sync = false}) => {
  const {theme} = useStyle();
  const frame = useCurrentFrame();
  const segments = parseRich(text).map((seg) => ({...seg, words: seg.text.split(/\s+/).filter(Boolean)}));
  const delays = syncDelays(segments.flatMap((s) => s.words), sync ? words : undefined, start, stagger);
  let k = 0;
  const nodes: React.ReactNode[] = [];
  segments.forEach((seg) =>
    seg.words.forEach((w) => {
      const i = k++;
      const p = progress(frame, delays[i], 16);
      nodes.push(
        <span
          key={i}
          style={{
            display: 'inline-block',
            whiteSpace: 'pre',
            fontStyle: seg.mark === 'italic' ? 'italic' : undefined,
            color: seg.mark === 'accent' || (seg.mark === 'italic' && serif) ? theme.accent : undefined,
            opacity: p,
            transform: `translateY(${(1 - p) * 0.25}em)`,
            filter: p < 0.98 ? `blur(${(1 - p) * 6}px)` : undefined,
          }}
        >
          {w}
        </span>,
      );
    }),
  );
  return (
    <div
      style={{
        fontFamily: serif ? fonts.serif : fonts.sans,
        fontSize: size,
        fontWeight: serif ? 400 : weight ?? 600,
        letterSpacing: serif ? '-0.015em' : '-0.02em',
        lineHeight: lineHeight ?? (serif ? 1.0 : 1.2),
        color: color ?? theme.text,
        textAlign: align,
        textWrap: 'balance',
      }}
    >
      {nodes.flatMap((n, i) => (i ? [' ', n] : [n]))}
    </div>
  );
};

// Wide-tracked uppercase label, like a section marker.
export const Label: React.FC<{text?: string; delay?: number; align?: 'left' | 'center'; style?: React.CSSProperties}> = ({text, delay = 0, align, style}) => {
  const {theme, layout} = useStyle();
  const p = useProgress(delay, 16);
  if (!text) return null;
  return (
    <div
      style={{
        fontFamily: fonts.sans,
        fontSize: layout.small,
        fontWeight: 600,
        letterSpacing: '0.2em',
        textTransform: 'uppercase',
        color: theme.muted,
        textAlign: align,
        ...enter(p, 10),
        ...style,
      }}
    >
      {text}
    </div>
  );
};

export const iconFor = (name?: string): LucideIcon | null => {
  if (!name) return null;
  const key = name
    .split(/[-_ ]/)
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join('') as keyof typeof icons;
  return icons[key] ?? null;
};

export const Icon: React.FC<{name?: string; size: number; color?: string; stroke?: number}> = ({name, size, color, stroke = 1.8}) => {
  const {theme} = useStyle();
  const Cmp = iconFor(name);
  if (!Cmp) return null;
  return <Cmp size={size} color={color ?? theme.accent} strokeWidth={stroke} absoluteStrokeWidth={false} style={{flex: 'none'}} />;
};

// A white card with an optional paper stack behind it.
export const Card: React.FC<{
  children: React.ReactNode;
  stacked?: boolean;
  dark?: boolean;
  p?: number;
  style?: React.CSSProperties;
}> = ({children, stacked = true, dark = false, p = 1, style}) => {
  const {theme} = useStyle();
  const face: React.CSSProperties = {
    background: dark ? theme.ink : theme.card,
    border: `1.5px solid ${dark ? theme.ink : theme.border}`,
    borderRadius: 22,
  };
  return (
    <div style={{position: 'relative', ...enter(p, 24), ...style}}>
      {stacked
        ? [12, 6].map((o) => (
            <div key={o} style={{position: 'absolute', inset: 0, transform: `translate(${o}px, ${o}px)`, ...face, background: theme.canvas, border: `1.5px solid ${theme.border}`, opacity: p}} />
          ))
        : null}
      <div style={{position: 'relative', height: '100%', boxSizing: 'border-box', boxShadow: theme.shadow, overflow: 'hidden', ...face}}>{children}</div>
    </div>
  );
};

export const CardHeader: React.FC<{icon?: string; title: string; dark?: boolean; right?: React.ReactNode}> = ({icon, title, dark, right}) => {
  const {theme, layout} = useStyle();
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: `${layout.body * 0.6}px ${layout.body * 0.75}px`,
        borderBottom: `1.5px solid ${dark ? 'rgba(255,255,255,0.08)' : theme.border}`,
        fontSize: layout.body * 0.82,
        fontWeight: 650,
        letterSpacing: '-0.01em',
        color: dark ? theme.inkText : theme.text,
      }}
    >
      <Icon name={icon} size={layout.body * 0.85} color={dark ? theme.inkText : theme.text} />
      <span style={{flex: 1}}>{title}</span>
      {right}
    </div>
  );
};

export const Pill: React.FC<{text: string; tone?: 'ink' | 'accent' | 'light'; icon?: string; p?: number; size?: number}> = ({text, tone = 'ink', icon, p = 1, size}) => {
  const {theme, layout} = useStyle();
  const fs = size ?? layout.body * 0.78;
  const colors =
    tone === 'ink'
      ? {bg: theme.ink, fg: theme.inkText, border: theme.ink}
      : tone === 'accent'
        ? {bg: theme.accent, fg: theme.accentInk, border: theme.accent}
        : {bg: theme.card, fg: theme.text, border: theme.border};
  return (
    <div
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: fs * 0.4,
        padding: `${fs * 0.42}px ${fs * 0.9}px`,
        borderRadius: 999,
        background: colors.bg,
        color: colors.fg,
        border: `1.5px solid ${colors.border}`,
        boxShadow: tone === 'light' ? theme.shadow : undefined,
        fontFamily: fonts.sans,
        fontSize: fs,
        fontWeight: 600,
        letterSpacing: '-0.01em',
        whiteSpace: 'nowrap',
        opacity: p,
        transform: `scale(${0.85 + 0.15 * p})`,
      }}
    >
      {icon ? <Icon name={icon} size={fs * 1.05} color={tone === 'light' ? theme.accent : colors.fg} stroke={2.2} /> : null}
      {text}
    </div>
  );
};

// A gently curved, hand-drawn style arrow that draws itself.
export const CurvedArrow: React.FC<{length: number; p: number; vertical?: boolean; bend?: number}> = ({length, p, vertical = false, bend = 0.18}) => {
  const {theme} = useStyle();
  const L = length;
  const h = Math.max(24, L * bend * 2 + 20);
  const path = `M 6 ${h / 2 + 4} Q ${L / 2} ${h / 2 - L * bend} ${L - 10} ${h / 2}`;
  const approx = L * 1.08;
  const head = p > 0.9 ? 1 : 0;
  const svg = (
    <svg width={L} height={h} viewBox={`0 0 ${L} ${h}`} style={{overflow: 'visible', display: 'block'}}>
      <path d={path} fill="none" stroke={theme.accent} strokeWidth={3.5} strokeLinecap="round" strokeDasharray={approx} strokeDashoffset={approx * (1 - p)} />
      <path
        d={`M ${L - 24} ${h / 2 - 11} L ${L - 9} ${h / 2} L ${L - 25} ${h / 2 + 9}`}
        fill="none"
        stroke={theme.accent}
        strokeWidth={3.5}
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity={head}
      />
    </svg>
  );
  return vertical ? <div style={{width: h, height: L, display: 'flex', alignItems: 'center', justifyContent: 'center'}}><div style={{transform: 'rotate(90deg)'}}>{svg}</div></div> : svg;
};

// Diagonal strike drawn across a rejected option.
export const Strike: React.FC<{p: number}> = ({p}) => {
  const {theme} = useStyle();
  return (
    <svg style={{position: 'absolute', inset: 0, width: '100%', height: '100%', overflow: 'visible'}} viewBox="0 0 100 100" preserveAspectRatio="none">
      <line
        x1="12"
        y1="12"
        x2={12 + 76 * p}
        y2={12 + 76 * p}
        stroke={theme.accent}
        strokeWidth={6}
        vectorEffect="non-scaling-stroke"
        strokeLinecap="round"
        opacity={p > 0 ? 1 : 0}
      />
    </svg>
  );
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

// Where a badge or label pill sits on a card: centred above it in a row of
// cards, pinned over the top-right corner when cards are stacked.
export const badgeSlot = (row: boolean, body: number): React.CSSProperties =>
  row
    ? {position: 'absolute', top: -body * 1.55, left: 0, right: 0, display: 'flex', justifyContent: 'center', zIndex: 2}
    : {position: 'absolute', top: -body * 0.6, right: body * 0.6, zIndex: 2};
