import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame} from 'remotion';
import {clamp, withAlpha} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneTiming} from '../types';

// Story-style segmented progress bar plus a small handle tag.
export const ProgressBar: React.FC<{scenes: SceneTiming[]; handle: string | null}> = ({scenes, handle}) => {
  const frame = useCurrentFrame();
  const {theme, layout} = useStyle();
  const top = layout.format === 'vertical' ? 120 : 48;
  const side = layout.format === 'vertical' ? 72 : 80;
  const intro = interpolate(frame, [0, 12], [0, 1], clamp);
  return (
    <AbsoluteFill style={{opacity: intro}}>
      <div style={{position: 'absolute', top, left: side, right: side, display: 'flex', gap: 10}}>
        {scenes.map((s, i) => {
          const p = interpolate(frame, [s.from, s.from + s.duration], [0, 1], clamp);
          return (
            <div
              key={i}
              style={{
                flex: Math.max(s.duration, 1),
                height: 8,
                borderRadius: 4,
                background: withAlpha(theme.dark ? '#FFFFFF' : '#000000', 0.12),
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  width: `${p * 100}%`,
                  height: '100%',
                  background: theme.accent,
                  borderRadius: 4,
                  boxShadow: `0 0 12px ${withAlpha(theme.accent, 0.8)}`,
                }}
              />
            </div>
          );
        })}
      </div>
      {handle ? (
        <div
          style={{
            position: 'absolute',
            top: top + 30,
            left: side,
            fontFamily: fonts.mono,
            fontSize: 28,
            fontWeight: 500,
            color: theme.muted,
            letterSpacing: '0.02em',
          }}
        >
          {handle}
        </div>
      ) : null}
    </AbsoluteFill>
  );
};

export const EXIT_FRAMES = 10;

// Positions a scene inside the content box and plays the shared exit move:
// the outgoing scene lifts, shrinks and defocuses while the next one enters.
export const SceneShell: React.FC<{scene: SceneTiming; children: React.ReactNode; last: boolean}> = ({
  scene,
  children,
  last,
}) => {
  const frame = useCurrentFrame();
  const {layout} = useStyle();
  const exit = last ? 0 : interpolate(frame, [scene.duration - 3, scene.duration + EXIT_FRAMES - 3], [0, 1], clamp);
  const eased = exit * exit;
  const drift = interpolate(frame, [0, scene.duration], [1, 1.025], clamp);
  return (
    <AbsoluteFill
      style={{
        opacity: 1 - eased,
        transform: `translateY(${-90 * eased}px) scale(${drift - 0.06 * eased})`,
        filter: eased > 0.001 ? `blur(${16 * eased}px)` : undefined,
      }}
    >
      <div
        style={{
          position: 'absolute',
          left: layout.content.x,
          top: layout.content.y,
          width: layout.content.w,
          height: layout.content.h,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
        }}
      >
        {children}
      </div>
    </AbsoluteFill>
  );
};
