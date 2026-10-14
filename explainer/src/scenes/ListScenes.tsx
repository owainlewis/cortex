import React from 'react';
import {useCurrentFrame} from 'remotion';
import {activeCue, formatNumber, Label, lift, MarkIcon, progress, RichText, Rule} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

const itemText = (item: any): string => (typeof item === 'string' ? item : item.text);

const Heading: React.FC<{kicker?: string; text?: string}> = ({kicker, text}) => {
  const {layout} = useStyle();
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 28, marginBottom: 20}}>
      <Label text={kicker} />
      {text ? <RichText text={text} size={layout.title} weight={650} start={2} lineHeight={1.06} /> : null}
    </div>
  );
};

// Numbered editorial list: serif numerals, hairline dividers, the item being
// spoken at full strength and earlier items stepped back.
export const List: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const items: any[] = d.items;
  const active = activeCue(scene.cues, frame);
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      {items.map((item, i) => {
        const at = scene.cues[i];
        const p = progress(frame, at, 18);
        const past = i < active;
        return (
          <div key={i} style={{display: 'flex', flexDirection: 'column'}}>
            <Rule delay={at - 4} />
            <div style={{display: 'flex', alignItems: 'baseline', gap: 36, padding: '30px 0', opacity: past ? 0.42 : 1}}>
              <div
                style={{
                  flex: 'none',
                  width: layout.body * 1.6,
                  fontFamily: fonts.serif,
                  fontStyle: 'italic',
                  fontSize: layout.body * 1.5,
                  lineHeight: 1,
                  color: theme.accent,
                  ...lift(p, 20),
                }}
              >
                {String(i + 1).padStart(2, '0')}
              </div>
              <RichText text={itemText(item)} size={layout.body * 1.05} weight={550} lineHeight={1.18} letterSpacing="-0.025em" start={at + 2} stagger={1} />
            </div>
          </div>
        );
      })}
      <Rule delay={scene.cues[items.length - 1] + 4} />
    </div>
  );
};

export const Compare: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const stacked = layout.format !== 'landscape';
  const sides = [d.left, d.right];
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <div style={{display: 'flex', flexDirection: stacked ? 'column' : 'row', gap: stacked ? 0 : 80}}>
        {sides.map((side, s) => {
          const at = scene.cues[s] ?? s * 20;
          const good = side.tone ? side.tone === 'good' : s === 1;
          const color = good ? theme.text : theme.muted;
          return (
            <div key={s} style={{flex: 1, display: 'flex', flexDirection: 'column'}}>
              <Rule delay={at - 4} color={good ? theme.accent : theme.line} thickness={good ? 3 : 2} />
              <Label
                text={side.label}
                delay={at}
                color={good ? theme.accent : theme.muted}
                style={{marginTop: 26, marginBottom: 22}}
              />
              <div style={{display: 'flex', flexDirection: 'column', gap: 20, marginBottom: 44}}>
                {(side.items as any[]).map((item, i) => {
                  const q = progress(frame, at + 5 + i * 4, 16);
                  return (
                    <div key={i} style={{display: 'flex', alignItems: 'center', gap: 22, ...lift(q, 14)}}>
                      <MarkIcon good={good} size={layout.body * 0.8} color={good ? theme.accent : theme.muted} />
                      <div
                        style={{
                          fontSize: layout.body,
                          fontWeight: good ? 600 : 450,
                          letterSpacing: '-0.02em',
                          lineHeight: 1.2,
                          color,
                          textDecoration: good ? undefined : 'line-through',
                          textDecorationColor: theme.faint,
                          textDecorationThickness: 2,
                        }}
                      >
                        {itemText(item)}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export const Bars: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const items: any[] = d.items;
  const max = Math.max(...items.map((x) => Number(x.value)));
  const decimals = d.decimals ?? 0;
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <div style={{display: 'flex', flexDirection: 'column', gap: 44}}>
        {items.map((item, i) => {
          const at = scene.cues[i] ?? i * 8;
          const appear = progress(frame, at, 14);
          const grow = progress(frame, at + 2, 30);
          const hl = !!item.highlight;
          const w = (Number(item.value) / max) * grow;
          return (
            <div key={i} style={{display: 'flex', flexDirection: 'column', gap: 16, ...lift(appear, 14)}}>
              <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'baseline'}}>
                <div style={{fontSize: layout.body, fontWeight: hl ? 600 : 450, letterSpacing: '-0.02em', color: hl ? theme.text : theme.muted}}>
                  {item.label}
                </div>
                <div style={{fontFamily: fonts.mono, fontSize: layout.body * 0.8, color: hl ? theme.accent : theme.muted, fontVariantNumeric: 'tabular-nums'}}>
                  {d.prefix ?? ''}
                  {formatNumber(Number(item.value) * grow, decimals)}
                  {d.unit ?? ''}
                </div>
              </div>
              <div style={{position: 'relative', height: 32}}>
                <div style={{position: 'absolute', left: 0, right: 0, top: 15, height: 2, background: theme.line}} />
                <div
                  style={{
                    position: 'absolute',
                    left: 0,
                    top: 0,
                    bottom: 0,
                    width: `${Math.max(w, 0.006) * 100}%`,
                    background: hl ? theme.accent : theme.muted,
                  }}
                />
              </div>
            </div>
          );
        })}
      </div>
      {d.note ? (
        <div style={{fontSize: layout.body * 0.85, color: theme.muted, marginTop: 40, ...lift(progress(frame, scene.cues[items.length] ?? 999, 16), 14)}}>
          {d.note}
        </div>
      ) : null}
    </div>
  );
};
