import React from 'react';
import {Easing, interpolate, useCurrentFrame} from 'remotion';
import {clamp, formatNumber, Kicker, riseStyle, RichText, springs, useSpringAt, withAlpha} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

export const Hook: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const d = scene.data;
  const frame = useCurrentFrame();
  const bar = useSpringAt(2, springs.smooth);
  const subAt = Math.min(scene.duration * 0.55, 30);
  const sub = useSpringAt(subAt, springs.smooth);
  // A quick punch-in on the first frames sells the "stop scrolling" moment.
  const punch = interpolate(frame, [0, 10], [1.08, 1], {...clamp, easing: Easing.out(Easing.cubic)});
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 44, transform: `scale(${punch})`}}>
      <Kicker text={d.kicker} />
      <RichText
        text={d.title}
        size={layout.titleSize * 1.32}
        weight={900}
        words={scene.words}
        sync
        start={2}
        lineHeight={1.0}
      />
      <div
        style={{
          height: 10,
          width: 220,
          borderRadius: 5,
          background: `linear-gradient(90deg, ${theme.accent}, ${withAlpha(theme.accent, 0)})`,
          transform: `scaleX(${bar})`,
          transformOrigin: 'left',
        }}
      />
      {d.subtitle ? (
        <div style={{fontSize: layout.bodySize, fontWeight: 600, color: theme.muted, lineHeight: 1.3, ...riseStyle(sub, 24, 8)}}>
          {d.subtitle}
        </div>
      ) : null}
    </div>
  );
};

export const Statement: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const d = scene.data;
  const subAt = scene.cues[0] ?? scene.duration * 0.6;
  const sub = useSpringAt(subAt, springs.smooth);
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 48}}>
      <Kicker text={d.kicker} />
      <RichText text={d.text} size={layout.titleSize * 0.82} weight={800} words={scene.words} sync lineHeight={1.08} />
      {d.sub ? (
        <div
          style={{
            fontSize: layout.bodySize,
            fontWeight: 600,
            color: theme.muted,
            lineHeight: 1.35,
            borderLeft: `6px solid ${theme.accent}`,
            paddingLeft: 28,
            ...riseStyle(sub, 24, 8),
          }}
        >
          {d.sub}
        </div>
      ) : null}
    </div>
  );
};

export const Stat: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const start = scene.cues[0] ?? 6;
  const countFrames = scene.extra.countFrames ?? 30;
  const t = interpolate(frame, [start, start + countFrames], [0, 1], {...clamp, easing: Easing.out(Easing.exp)});
  const value = Number(d.value) * t;
  const appear = useSpringAt(start - 4, springs.bouncy);
  const land = useSpringAt(start + countFrames, springs.bouncy);
  const label = useSpringAt(start + 8, springs.smooth);
  const subAt = scene.cues[1] ?? start + countFrames + 10;
  const sub = useSpringAt(subAt, springs.smooth);
  const digits = formatNumber(Number(d.value), d.decimals ?? 0).length + (d.prefix ?? '').length + (d.suffix ?? '').length;
  const size = Math.min(layout.titleSize * 2.9, layout.content.w / (Math.max(digits, 3) * 0.62));
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 36}}>
      <Kicker text={d.kicker} />
      <div
        style={{
          fontSize: size,
          fontWeight: 900,
          letterSpacing: '-0.05em',
          lineHeight: 0.95,
          fontVariantNumeric: 'tabular-nums',
          whiteSpace: 'nowrap',
          color: theme.accent,
          textShadow: theme.dark ? `0 0 80px ${withAlpha(theme.accent, 0.45)}` : undefined,
          transform: `scale(${interpolate(appear, [0, 1], [0.6, 1]) * interpolate(land, [0, 0.5, 1], [1, 1.06, 1])})`,
          transformOrigin: 'left bottom',
          opacity: interpolate(appear, [0, 0.3], [0, 1], clamp),
        }}
      >
        {d.prefix ?? ''}
        {formatNumber(value, d.decimals ?? 0)}
        {d.suffix ?? ''}
      </div>
      <div
        style={{
          height: 14,
          borderRadius: 7,
          background: withAlpha(theme.dark ? '#FFFFFF' : '#000000', 0.08),
          overflow: 'hidden',
        }}
      >
        <div style={{width: `${t * 100}%`, height: '100%', background: theme.accent, borderRadius: 7}} />
      </div>
      <div style={{...riseStyle(label, 30, 8)}}>
        <RichText text={d.label} size={layout.titleSize * 0.62} weight={800} start={start + 8} stagger={2} />
      </div>
      {d.sub ? (
        <div style={{fontSize: layout.bodySize, fontWeight: 600, color: theme.muted, lineHeight: 1.35, ...riseStyle(sub, 24, 8)}}>
          {d.sub}
        </div>
      ) : null}
    </div>
  );
};

export const Outro: React.FC<SceneProps & {handle?: string | null}> = ({scene, handle}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const pill = useSpringAt(Math.min(scene.duration * 0.45, 24), springs.bouncy);
  const glow = 0.5 + 0.5 * Math.sin(frame / 8);
  return (
    <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 56, textAlign: 'center'}}>
      <RichText text={d.title} size={layout.titleSize} weight={900} words={scene.words} sync align="center" />
      {d.sub ? (
        <div style={{fontSize: layout.bodySize, fontWeight: 600, color: theme.muted, lineHeight: 1.35, ...riseStyle(pill, 20, 6)}}>
          {d.sub}
        </div>
      ) : null}
      {d.cta || handle ? (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 20,
            padding: '26px 48px',
            borderRadius: 999,
            background: theme.accent,
            color: theme.accentInk,
            fontSize: layout.bodySize,
            fontWeight: 800,
            boxShadow: `0 0 ${40 + 40 * glow}px ${withAlpha(theme.accent, 0.5)}`,
            transform: `scale(${pill})`,
          }}
        >
          <span>{d.cta ?? 'Follow'}</span>
          {handle ? <span style={{fontFamily: fonts.mono, fontWeight: 700, opacity: 0.75}}>{handle}</span> : null}
        </div>
      ) : null}
    </div>
  );
};
