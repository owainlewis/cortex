import React from 'react';
import {Easing, interpolate, useCurrentFrame} from 'remotion';
import {clamp, formatNumber, Label, lift, progress, RichText, Rule, useProgress, ArrowIcon} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

export const Hook: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const d = scene.data;
  const subAt = Math.min(scene.duration * 0.6, 34);
  const sub = useProgress(subAt, 18);
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 48}}>
      <Label text={d.kicker} color={theme.accent} />
      <RichText text={d.title} size={layout.display * 1.12} weight={700} words={scene.words} sync start={1} lineHeight={0.98} />
      {d.subtitle ? (
        <div style={{display: 'flex', flexDirection: 'column', gap: 28}}>
          <Rule delay={subAt - 6} />
          <div style={{fontSize: layout.body, fontWeight: 450, color: theme.muted, lineHeight: 1.3, ...lift(sub, 16)}}>{d.subtitle}</div>
        </div>
      ) : null}
    </div>
  );
};

export const Statement: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const d = scene.data;
  const subAt = scene.cues[0] ?? scene.duration * 0.6;
  const sub = useProgress(subAt, 18);
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 48}}>
      <Label text={d.kicker} />
      <RichText text={d.text} size={layout.display * 0.8} weight={650} words={scene.words} sync lineHeight={1.04} />
      {d.sub ? (
        <div style={{display: 'flex', flexDirection: 'column', gap: 28}}>
          <Rule delay={subAt - 4} />
          <div style={{fontSize: layout.body, fontWeight: 450, color: theme.muted, lineHeight: 1.32, ...lift(sub, 16)}}>{d.sub}</div>
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
  const appear = progress(frame, start - 2, 12);
  const subAt = scene.cues[1] ?? start + countFrames + 10;
  const sub = progress(frame, subAt, 18);
  const number = formatNumber(Number(d.value) * t, d.decimals ?? 0);
  const chars = formatNumber(Number(d.value), d.decimals ?? 0).length + (d.prefix ?? '').length + (d.suffix ?? '').length * 0.7;
  const size = Math.min(layout.display * 2.6, layout.content.w / (Math.max(chars, 2.5) * 0.6));
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 36}}>
      <Label text={d.kicker} />
      <div
        style={{
          fontSize: size,
          fontWeight: 700,
          letterSpacing: '-0.06em',
          lineHeight: 0.9,
          fontVariantNumeric: 'tabular-nums',
          whiteSpace: 'nowrap',
          ...lift(appear, 30),
        }}
      >
        {d.prefix ?? ''}
        {number}
        {d.suffix ? (
          <span style={{fontFamily: fonts.serif, fontStyle: 'italic', fontWeight: 400, letterSpacing: '-0.02em', color: theme.accent}}>
            {d.suffix}
          </span>
        ) : null}
      </div>
      <Rule delay={start} color={theme.text} thickness={3} />
      <RichText text={d.label} size={layout.title} weight={600} start={start + 6} lineHeight={1.08} />
      {d.sub ? <div style={{fontSize: layout.body, fontWeight: 450, color: theme.muted, lineHeight: 1.32, ...lift(sub, 16)}}>{d.sub}</div> : null}
    </div>
  );
};

export const Outro: React.FC<SceneProps & {handle?: string | null}> = ({scene, handle}) => {
  const {layout, theme} = useStyle();
  const d = scene.data;
  const at = Math.min(scene.duration * 0.45, 24);
  const cta = useProgress(at, 18);
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 48}}>
      <RichText text={d.title} size={layout.display} weight={700} words={scene.words} sync lineHeight={1.0} />
      <Rule delay={at - 6} />
      {d.sub ? <div style={{fontSize: layout.body, fontWeight: 450, color: theme.muted, lineHeight: 1.32, ...lift(cta, 16)}}>{d.sub}</div> : null}
      {d.cta || handle ? (
        <div style={{display: 'flex', alignItems: 'center', gap: 24, ...lift(cta, 16)}}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 18,
              padding: '22px 34px',
              background: theme.accent,
              color: theme.accentInk,
              fontSize: layout.body * 0.9,
              fontWeight: 650,
              letterSpacing: '-0.02em',
            }}
          >
            {d.cta ?? 'Follow'}
            <ArrowIcon size={layout.body * 0.8} color={theme.accentInk} />
          </div>
          {handle ? <span style={{fontFamily: fonts.mono, fontSize: layout.small * 1.2, color: theme.muted}}>{handle}</span> : null}
        </div>
      ) : null}
    </div>
  );
};
