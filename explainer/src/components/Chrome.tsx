import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame} from 'remotion';
import {clamp, easeIn, progress} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneTiming} from '../types';

// Top of frame: a hairline progress bar and a section counter.
export const TopBar: React.FC<{scenes: SceneTiming[]; handle: string | null}> = ({scenes, handle}) => {
  const frame = useCurrentFrame();
  const {theme, layout} = useStyle();
  const total = scenes[scenes.length - 1].from + scenes[scenes.length - 1].duration;
  let current = 0;
  scenes.forEach((s, i) => {
    if (frame >= s.from) current = i;
  });
  const top = layout.format === 'vertical' ? 150 : 56;
  const intro = progress(frame, 0, 12);
  return (
    <AbsoluteFill>
      <div style={{position: 'absolute', top: 0, left: 0, height: 6, width: `${(frame / total) * 100}%`, background: theme.accent}} />
      <div
        style={{
          position: 'absolute',
          top,
          left: layout.margin,
          right: layout.margin,
          display: 'flex',
          justifyContent: 'space-between',
          fontFamily: fonts.mono,
          fontSize: layout.small,
          letterSpacing: '0.04em',
          color: theme.muted,
          opacity: intro,
        }}
      >
        <span>
          <span style={{color: theme.text}}>{String(current + 1).padStart(2, '0')}</span>
          {' / '}
          {String(scenes.length).padStart(2, '0')}
        </span>
        {handle ? <span>{handle}</span> : null}
      </div>
    </AbsoluteFill>
  );
};

export const EXIT_FRAMES = 8;

// Positions a scene in the content box. Scenes leave with a short ease-in
// lift so the next one can enter on a clean frame.
export const SceneShell: React.FC<{scene: SceneTiming; children: React.ReactNode; last: boolean}> = ({scene, children, last}) => {
  const frame = useCurrentFrame();
  const {layout} = useStyle();
  const exit = last ? 0 : interpolate(frame, [scene.duration - EXIT_FRAMES, scene.duration], [0, 1], {...clamp, easing: easeIn});
  return (
    <AbsoluteFill style={{opacity: 1 - exit, transform: `translateY(${-40 * exit}px)`}}>
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
