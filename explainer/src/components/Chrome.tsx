import React from 'react';
import {AbsoluteFill, interpolate, OffthreadVideo, staticFile, useCurrentFrame} from 'remotion';
import {clamp, easeIn, easeOut} from '../motion';
import {fonts, SceneIndex, useStyle} from '../theme';
import type {SceneTiming} from '../types';

// The soft outer frame and the rounded canvas everything sits on.
export const Frame: React.FC<{children: React.ReactNode}> = ({children}) => {
  const {theme, layout} = useStyle();
  return (
    <AbsoluteFill style={{background: theme.frame}}>
      <div
        style={{
          position: 'absolute',
          left: layout.canvas.x,
          top: layout.canvas.y,
          width: layout.canvas.w,
          height: layout.canvas.h,
          borderRadius: layout.canvas.radius,
          background: theme.canvas,
          overflow: 'hidden',
          boxShadow: theme.dark || layout.split ? undefined : '0 1px 0 rgba(255,255,255,0.8) inset, 0 2px 12px rgba(30,28,20,0.06)',
        }}
      >
        {theme.background === 'grid' ? <DraftingGrid /> : null}
        {theme.background === 'dots' ? (
          <div
            style={{
              position: 'absolute',
              inset: 0,
              backgroundImage: `radial-gradient(${theme.faint} 1.5px, transparent 1.6px)`,
              backgroundSize: '30px 30px',
              opacity: 0.55,
              maskImage: 'radial-gradient(ellipse 75% 70% at 50% 50%, black 30%, transparent 100%)',
              WebkitMaskImage: 'radial-gradient(ellipse 75% 70% at 50% 50%, black 30%, transparent 100%)',
            }}
          />
        ) : null}
        <div style={{position: 'absolute', left: -layout.canvas.x, top: -layout.canvas.y, width: layout.width, height: layout.height}}>{children}</div>
      </div>
    </AbsoluteFill>
  );
};

// Technical-paper background: a faint fine grid, margin rules around the
// content area, and registration crosshairs where they meet.
const DraftingGrid: React.FC = () => {
  const {theme, layout} = useStyle();
  const {x, y, w, h} = layout.canvas;
  const c = layout.content;
  const left = c.x - 36 - x;
  const right = c.x + c.w + 36 - x;
  const top = Math.max(24, c.y - 40 - y);
  const bottom = Math.min(h - 24, c.y + c.h + 24 - y);
  const cross = (cx: number, cy: number) => (
    <g key={`${cx}-${cy}`} stroke={theme.muted} strokeWidth={1.5}>
      <line x1={cx - 9} y1={cy} x2={cx + 9} y2={cy} />
      <line x1={cx} y1={cy - 9} x2={cx} y2={cy + 9} />
    </g>
  );
  return (
    <svg width={w} height={h} style={{position: 'absolute', inset: 0}}>
      <defs>
        <pattern id="fine-grid" width="40" height="40" patternUnits="userSpaceOnUse">
          <path d="M 40 0 L 0 0 0 40" fill="none" stroke={theme.border} strokeWidth="1" />
        </pattern>
      </defs>
      <rect width={w} height={h} fill="url(#fine-grid)" opacity={0.35} />
      <g stroke={theme.border} strokeWidth={1.5}>
        <line x1={left} y1={0} x2={left} y2={h} />
        <line x1={right} y1={0} x2={right} y2={h} />
        <line x1={0} y1={top} x2={w} y2={top} />
        <line x1={0} y1={bottom} x2={w} y2={bottom} />
      </g>
      {[cross(left, top), cross(right, top), cross(left, bottom), cross(right, bottom)]}
    </svg>
  );
};

export const EXIT_FRAMES = 10;

// Places a scene in the content box; scenes leave with a short soft fade.
export const SceneShell: React.FC<{scene: SceneTiming; children: React.ReactNode; last: boolean; index?: number; total?: number}> = ({
  scene,
  children,
  last,
  index = 0,
  total = 1,
}) => {
  const frame = useCurrentFrame();
  const {layout} = useStyle();
  const exit = last ? 0 : interpolate(frame, [scene.duration - EXIT_FRAMES, scene.duration], [0, 1], {...clamp, easing: easeIn});
  return (
    <AbsoluteFill style={{opacity: 1 - exit, transform: `scale(${1 - 0.02 * exit})`}}>
      <div
        style={{
          position: 'absolute',
          left: layout.content.x,
          top: layout.content.y,
          width: layout.content.w,
          height: layout.content.h,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'safe center',
        }}
      >
        <SceneIndex.Provider value={{index, total}}>{children}</SceneIndex.Provider>
      </div>
    </AbsoluteFill>
  );
};

const TRANSITION = 14;

// How far into picture-in-picture the speaker is: 0 full frame, 1 tucked in
// the corner. Scenes separated by short gaps are treated as one stretch.
export const pipAmount = (scenes: SceneTiming[], frame: number) => {
  const spans: [number, number][] = [];
  for (const s of scenes) {
    const last = spans[spans.length - 1];
    if (last && s.from - last[1] < 20) last[1] = s.from + s.duration;
    else spans.push([s.from, s.from + s.duration]);
  }
  let amount = 0;
  for (const [a, b] of spans) {
    const inP = interpolate(frame, [a - TRANSITION / 2, a + TRANSITION / 2], [0, 1], {...clamp, easing: easeOut});
    const outP = interpolate(frame, [b - TRANSITION / 2, b + TRANSITION / 2], [0, 1], {...clamp, easing: easeIn});
    amount = Math.max(amount, Math.min(inP, 1 - outP));
  }
  return amount;
};

// The presenter: full frame between graphics, a framed card beside them.
export const Speaker: React.FC<{src: string; scenes: SceneTiming[]; focus?: [number, number]}> = ({src, scenes, focus = [0.5, 0.4]}) => {
  const frame = useCurrentFrame();
  const {layout, theme} = useStyle();
  // In a split screen the speaker keeps the bottom panel the whole time.
  const t = layout.split ? 1 : pipAmount(scenes, frame);
  const full = {x: layout.inset, y: layout.inset, w: layout.width - layout.inset * 2, h: layout.height - layout.inset * 2};
  const pip = layout.pip;
  const lerp = (a: number, b: number) => a + (b - a) * t;
  return (
    <div
      style={{
        position: 'absolute',
        left: lerp(full.x, pip.x),
        top: lerp(full.y, pip.y),
        width: lerp(full.w, pip.w),
        height: lerp(full.h, pip.h),
        borderRadius: layout.split ? 0 : lerp(layout.radius, 22),
        overflow: 'hidden',
        boxShadow: t > 0.01 && !layout.split ? theme.shadow : undefined,
        border: t > 0.01 && !layout.split ? `${3 * t}px solid ${theme.card}` : undefined,
        background: '#000',
      }}
    >
      <OffthreadVideo src={staticFile(src)} muted style={{width: '100%', height: '100%', objectFit: 'cover', objectPosition: `${focus[0] * 100}% ${focus[1] * 100}%`}} />
    </div>
  );
};

// What the top panel of a split screen shows between graphics: the video's
// title, so the panel is never empty.
export const IdleTitle: React.FC<{title: string; scenes: SceneTiming[]; handle: string | null}> = ({title, scenes, handle}) => {
  const frame = useCurrentFrame();
  const {layout, theme} = useStyle();
  const show = 1 - pipAmount(scenes, frame);
  if (!layout.split || show < 0.01) return null;
  return (
    <div
      style={{
        position: 'absolute',
        left: layout.content.x,
        top: layout.content.y,
        width: layout.content.w,
        height: layout.content.h,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        textAlign: 'center',
        gap: 22,
        opacity: show,
      }}
    >
      {handle ? (
        <div style={{fontFamily: fonts.sans, fontSize: layout.small, fontWeight: 600, letterSpacing: '0.2em', textTransform: 'uppercase', color: theme.muted}}>{handle}</div>
      ) : null}
      <div style={{fontFamily: fonts.serif, fontSize: layout.display * 0.8, lineHeight: 1.02, letterSpacing: '-0.015em', color: theme.text, textWrap: 'balance'}}>{title}</div>
    </div>
  );
};
