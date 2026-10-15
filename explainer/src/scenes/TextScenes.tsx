import React from 'react';
import {Easing, interpolate, useCurrentFrame} from 'remotion';
import {Card, clamp, Divider, formatNumber, Label, Pill, progress, RichText, useProgress} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

const Center: React.FC<{children: React.ReactNode; gap?: number}> = ({children, gap = 40}) => (
  <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', textAlign: 'center', gap}}>{children}</div>
);

export const Hook: React.FC<SceneProps> = ({scene}) => {
  const {layout} = useStyle();
  const d = scene.data;
  const subAt = scene.cues[0] ?? Math.min(scene.duration * 0.6, 36);
  const sub = useProgress(subAt, 14);
  return (
    <Center>
      <Label text={d.kicker} align="center" />
      <RichText text={d.title} serif size={layout.display * 1.05} words={scene.words} sync start={1} align="center" />
      <Divider delay={14} />
      {d.subtitle ? <Pill text={d.subtitle} p={sub} /> : null}
    </Center>
  );
};

export const Statement: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const d = scene.data;
  const subAt = scene.cues[0] ?? scene.duration * 0.6;
  const sub = useProgress(subAt, 14);
  const short = d.sub && String(d.sub).length <= 36;
  return (
    <Center>
      <Label text={d.kicker} align="center" />
      <RichText text={d.text} serif size={layout.display * 0.85} words={scene.words} sync align="center" />
      <Divider delay={12} />
      {d.sub ? (
        short ? (
          <Pill text={d.sub} p={sub} />
        ) : (
          <div style={{fontSize: layout.body, color: theme.muted, lineHeight: 1.35, maxWidth: layout.content.w * 0.8, opacity: sub}}>{d.sub}</div>
        )
      ) : null}
    </Center>
  );
};

export const Stat: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const start = scene.cues[0] ?? 6;
  const countFrames = scene.extra.countFrames ?? 30;
  const t = interpolate(frame, [start, start + countFrames], [0, 1], {...clamp, easing: Easing.out(Easing.exp)});
  const card = progress(frame, Math.max(0, start - 8), 18);
  const subAt = scene.cues[1] ?? start + countFrames + 10;
  const sub = progress(frame, subAt, 14);
  const chars = formatNumber(Number(d.value), d.decimals ?? 0).length + (d.prefix ?? '').length + (d.suffix ?? '').length * 0.6;
  const size = Math.min(layout.display * 2.2, (layout.content.w * 0.7) / (Math.max(chars, 2.5) * 0.48));
  return (
    <Center gap={36}>
      <Label text={d.kicker} align="center" />
      <Card p={card} style={{minWidth: layout.content.w * 0.55}}>
        <div style={{padding: `${layout.body}px ${layout.body * 1.6}px ${layout.body * 1.2}px`, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 18}}>
          <div style={{fontFamily: fonts.serif, fontSize: size, lineHeight: 1, letterSpacing: '-0.02em', fontVariantNumeric: 'tabular-nums', whiteSpace: 'nowrap'}}>
            {d.prefix ?? ''}
            {formatNumber(Number(d.value) * t, d.decimals ?? 0)}
            {d.suffix ? <span style={{fontStyle: 'italic', color: theme.accent}}>{d.suffix}</span> : null}
          </div>
          <RichText text={d.label} size={layout.body} weight={600} start={start + 6} align="center" />
        </div>
      </Card>
      {d.sub ? <Pill text={d.sub} p={sub} /> : null}
    </Center>
  );
};

export const Outro: React.FC<SceneProps & {handle?: string | null}> = ({scene, handle}) => {
  const {layout, theme} = useStyle();
  const d = scene.data;
  const at = Math.min(scene.duration * 0.45, 24);
  const cta = useProgress(at, 14);
  return (
    <Center>
      <RichText text={d.title} serif size={layout.display} words={scene.words} sync align="center" />
      <Divider delay={12} />
      {d.sub ? <div style={{fontSize: layout.body, color: theme.muted, opacity: cta}}>{d.sub}</div> : null}
      {d.cta || handle ? <Pill text={[d.cta ?? 'Follow', handle].filter(Boolean).join('  ')} icon="arrow-right" p={cta} size={layout.body * 0.9} /> : null}
    </Center>
  );
};
