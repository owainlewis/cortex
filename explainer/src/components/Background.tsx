import React from 'react';
import {AbsoluteFill, interpolate, random, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {springs, withAlpha} from '../motion';
import {useStyle} from '../theme';
import type {SceneTiming} from '../types';

// Blob anchor for a given scene, as fractions of the frame.
const anchor = (scene: number, blob: number) => ({
  x: 0.15 + random(`x-${scene}-${blob}`) * 0.7,
  y: 0.1 + random(`y-${scene}-${blob}`) * 0.8,
});

export const Background: React.FC<{scenes: SceneTiming[]}> = ({scenes}) => {
  const frame = useCurrentFrame();
  const {fps, width, height} = useVideoConfig();
  const {theme} = useStyle();

  // Ease every blob from the previous scene's anchor to the current one, so
  // each scene change also shifts the lighting.
  let current = 0;
  scenes.forEach((s, i) => {
    if (frame >= s.from) current = i;
  });
  const sceneStart = scenes[current]?.from ?? 0;
  const move = spring({frame: frame - sceneStart, fps, config: {damping: 200, stiffness: 40, mass: 1.4}});

  const colors = [theme.glowA, theme.glowB, theme.glowC];
  const blobs = colors.map((color, b) => {
    const from = anchor(Math.max(current - 1, 0), b);
    const to = anchor(current, b);
    const t = current === 0 ? 1 : move;
    const drift = frame / fps;
    const x = (from.x + (to.x - from.x) * t) * width + Math.sin(drift * 0.35 + b * 2) * 60;
    const y = (from.y + (to.y - from.y) * t) * height + Math.cos(drift * 0.28 + b) * 80;
    const size = Math.max(width, height) * (0.62 + b * 0.08);
    return (
      <div
        key={b}
        style={{
          position: 'absolute',
          left: x - size / 2,
          top: y - size / 2,
          width: size,
          height: size,
          borderRadius: '50%',
          background: `radial-gradient(circle, ${withAlpha(color, theme.dark ? 0.55 : 0.9)} 0%, ${withAlpha(
            color,
            0,
          )} 62%)`,
        }}
      />
    );
  });

  const gridShift = (frame * 0.4) % 80;
  const grain = Math.floor(frame / 2) % 24;
  const pulse = spring({frame: frame - sceneStart, fps, config: springs.smooth});

  return (
    <AbsoluteFill style={{background: `linear-gradient(180deg, ${theme.bg2} 0%, ${theme.bg} 100%)`}}>
      {blobs}
      <AbsoluteFill
        style={{
          backgroundImage: `linear-gradient(${withAlpha(theme.dark ? '#FFFFFF' : '#000000', 0.045)} 2px, transparent 2px), linear-gradient(90deg, ${withAlpha(
            theme.dark ? '#FFFFFF' : '#000000',
            0.045,
          )} 2px, transparent 2px)`,
          backgroundSize: '80px 80px',
          backgroundPosition: `0px ${gridShift}px`,
          maskImage: 'radial-gradient(ellipse 70% 55% at 50% 42%, black 10%, transparent 75%)',
          WebkitMaskImage: 'radial-gradient(ellipse 70% 55% at 50% 42%, black 10%, transparent 75%)',
          opacity: interpolate(pulse, [0, 1], [1.6, 1]),
        }}
      />
      <AbsoluteFill
        style={{
          background: theme.dark
            ? 'radial-gradient(ellipse 90% 75% at 50% 45%, transparent 45%, rgba(0,0,0,0.6) 100%)'
            : 'radial-gradient(ellipse 90% 75% at 50% 45%, transparent 55%, rgba(120,100,60,0.18) 100%)',
        }}
      />
      <AbsoluteFill style={{opacity: theme.dark ? 0.09 : 0.06, mixBlendMode: 'overlay'}}>
        {/* Half-resolution noise scaled up: cheaper, and a more filmic grain size. */}
        <svg width={width / 2} height={height / 2} style={{transform: 'scale(2)', transformOrigin: '0 0'}}>
          <filter id="grain">
            <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed={grain} stitchTiles="stitch" />
            <feColorMatrix type="saturate" values="0" />
          </filter>
          <rect width="100%" height="100%" filter="url(#grain)" />
        </svg>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};
