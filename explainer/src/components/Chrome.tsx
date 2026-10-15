import React from 'react';
import {AbsoluteFill, interpolate, OffthreadVideo, staticFile, useCurrentFrame} from 'remotion';
import {clamp, easeIn, easeOut} from '../motion';
import {useStyle} from '../theme';
import type {SceneTiming} from '../types';

// The soft outer frame and the rounded canvas everything sits on.
export const Frame: React.FC<{children: React.ReactNode}> = ({children}) => {
  const {theme, layout} = useStyle();
  return (
    <AbsoluteFill style={{background: theme.frame}}>
      <div
        style={{
          position: 'absolute',
          inset: layout.inset,
          borderRadius: layout.radius,
          background: theme.canvas,
          overflow: 'hidden',
          boxShadow: theme.dark ? undefined : '0 1px 0 rgba(255,255,255,0.8) inset, 0 2px 12px rgba(30,28,20,0.06)',
        }}
      >
        <div style={{position: 'absolute', inset: -layout.inset}}>{children}</div>
      </div>
    </AbsoluteFill>
  );
};

export const EXIT_FRAMES = 10;

// Places a scene in the content box; scenes leave with a short soft fade.
export const SceneShell: React.FC<{scene: SceneTiming; children: React.ReactNode; last: boolean}> = ({scene, children, last}) => {
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
        {children}
      </div>
    </AbsoluteFill>
  );
};

const TRANSITION = 14;

// How far into picture-in-picture the speaker is: 0 full frame, 1 tucked in
// the corner. Scenes separated by short gaps are treated as one stretch.
const pipAmount = (scenes: SceneTiming[], frame: number) => {
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
  const t = pipAmount(scenes, frame);
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
        borderRadius: lerp(layout.radius, 22),
        overflow: 'hidden',
        boxShadow: t > 0.01 ? theme.shadow : undefined,
        border: t > 0.01 ? `${3 * t}px solid ${theme.card}` : undefined,
        background: '#000',
      }}
    >
      <OffthreadVideo src={staticFile(src)} muted style={{width: '100%', height: '100%', objectFit: 'cover', objectPosition: `${focus[0] * 100}% ${focus[1] * 100}%`}} />
    </div>
  );
};
