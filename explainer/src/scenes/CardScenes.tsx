import React from 'react';
import {interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {activeCue, clamp, formatNumber, Kicker, Panel, riseStyle, RichText, springs, withAlpha} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

const itemText = (item: any): string => (typeof item === 'string' ? item : item.text);

const Heading: React.FC<{text?: string; size: number}> = ({text, size}) =>
  text ? <RichText text={text} size={size} weight={800} start={2} stagger={2} /> : null;

export const List: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const d = scene.data;
  const items: any[] = d.items;
  const active = activeCue(scene.cues, frame);
  const compact = items.length > 4;
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: compact ? 22 : 30}}>
      <Kicker text={d.kicker} />
      <Heading text={d.heading} size={layout.titleSize * 0.66} />
      <div style={{height: 8}} />
      {items.map((item, i) => {
        const p = spring({frame: frame - scene.cues[i], fps, config: springs.snappy});
        const badge = spring({frame: frame - scene.cues[i] - 3, fps, config: springs.bouncy});
        const isActive = i === active;
        const past = i < active;
        return (
          <div
            key={i}
            style={{
              opacity: interpolate(p, [0, 0.5], [0, past ? 0.5 : 1], clamp) * (past ? 1 : 1),
              transform: `translateX(${(1 - p) * 90}px) scale(${isActive ? 1 : 0.97})`,
              transformOrigin: 'left center',
              filter: p < 0.99 ? `blur(${(1 - p) * 10}px)` : undefined,
            }}
          >
            <Panel
              glow={isActive ? 1 : 0}
              style={{display: 'flex', alignItems: 'center', gap: 30, padding: compact ? '24px 32px' : '32px 36px'}}
            >
              <div
                style={{
                  flex: 'none',
                  width: compact ? 64 : 76,
                  height: compact ? 64 : 76,
                  borderRadius: 20,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  fontFamily: fonts.mono,
                  fontSize: compact ? 28 : 32,
                  fontWeight: 700,
                  background: isActive ? theme.accent : withAlpha(theme.accent, 0.12),
                  color: isActive ? theme.accentInk : theme.accent,
                  transform: `scale(${badge}) rotate(${(1 - badge) * -40}deg)`,
                }}
              >
                {String(i + 1).padStart(2, '0')}
              </div>
              <RichText
                text={itemText(item)}
                size={compact ? layout.bodySize * 0.92 : layout.bodySize}
                weight={700}
                lineHeight={1.2}
                letterSpacing="-0.015em"
                start={scene.cues[i] + 2}
                stagger={1}
              />
            </Panel>
          </div>
        );
      })}
    </div>
  );
};

const Mark: React.FC<{good: boolean; size: number}> = ({good, size}) => {
  const {theme} = useStyle();
  const color = good ? theme.good : theme.bad;
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" style={{flex: 'none'}}>
      <circle cx="12" cy="12" r="11" fill={withAlpha(color, 0.16)} stroke={color} strokeWidth="1.5" />
      {good ? (
        <path d="M7 12.5l3.2 3.2L17 9" fill="none" stroke={color} strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />
      ) : (
        <path d="M8.5 8.5l7 7M15.5 8.5l-7 7" fill="none" stroke={color} strokeWidth="2.4" strokeLinecap="round" />
      )}
    </svg>
  );
};

export const Compare: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const d = scene.data;
  const sides = [d.left, d.right];
  const stacked = layout.format === 'vertical';
  const vs = spring({frame: frame - (scene.cues[1] ?? 20) + 4, fps, config: springs.bouncy});
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 30}}>
      <Kicker text={d.kicker} />
      <Heading text={d.heading} size={layout.titleSize * 0.62} />
      <div style={{position: 'relative', display: 'flex', flexDirection: stacked ? 'column' : 'row', gap: stacked ? 44 : 50, marginTop: 10}}>
        {sides.map((side, s) => {
          const at = scene.cues[s] ?? s * 20;
          const p = spring({frame: frame - at, fps, config: springs.snappy});
          const good = side.tone ? side.tone === 'good' : s === 1;
          const color = good ? theme.good : theme.bad;
          return (
            <div key={s} style={{flex: 1, ...riseStyle(p, 70, 12)}}>
              <Panel style={{padding: '34px 38px', borderTop: `6px solid ${color}`}}>
                <div
                  style={{
                    fontSize: layout.bodySize * 0.62,
                    fontWeight: 800,
                    letterSpacing: '0.14em',
                    textTransform: 'uppercase',
                    color,
                    marginBottom: 22,
                  }}
                >
                  {side.label}
                </div>
                <div style={{display: 'flex', flexDirection: 'column', gap: 18}}>
                  {(side.items as any[]).map((item, i) => {
                    const q = spring({frame: frame - at - 6 - i * 5, fps, config: springs.snappy});
                    return (
                      <div key={i} style={{display: 'flex', alignItems: 'center', gap: 20, ...riseStyle(q, 24, 6)}}>
                        <Mark good={good} size={layout.bodySize * 0.95} />
                        <div style={{fontSize: layout.bodySize * 0.88, fontWeight: 700, lineHeight: 1.2, color: theme.text}}>
                          {itemText(item)}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </Panel>
            </div>
          );
        })}
        <div
          style={{
            position: 'absolute',
            left: '50%',
            top: '50%',
            width: 96,
            height: 96,
            marginLeft: -48,
            marginTop: -48,
            borderRadius: 48,
            background: theme.accent,
            color: theme.accentInk,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 36,
            fontWeight: 900,
            transform: `scale(${vs}) rotate(${(1 - vs) * 90}deg)`,
            boxShadow: `0 0 40px ${withAlpha(theme.accent, 0.5)}`,
          }}
        >
          VS
        </div>
      </div>
    </div>
  );
};

export const Bars: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const d = scene.data;
  const items: any[] = d.items;
  const max = Math.max(...items.map((x) => Number(x.value)));
  const decimals = d.decimals ?? 0;
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 30}}>
      <Kicker text={d.kicker} />
      <Heading text={d.heading} size={layout.titleSize * 0.62} />
      <div style={{display: 'flex', flexDirection: 'column', gap: 30, marginTop: 16}}>
        {items.map((item, i) => {
          const at = scene.cues[i] ?? i * 8;
          const appear = spring({frame: frame - at, fps, config: springs.smooth});
          const grow = spring({frame: frame - at - 3, fps, config: {damping: 26, stiffness: 90, mass: 1}});
          const hl = !!item.highlight;
          const color = hl ? theme.accent : withAlpha(theme.dark ? '#FFFFFF' : '#000000', 0.22);
          const w = (Number(item.value) / max) * grow;
          return (
            <div key={i} style={{display: 'flex', flexDirection: 'column', gap: 12, ...riseStyle(appear, 20, 6)}}>
              <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'baseline'}}>
                <div style={{fontSize: layout.bodySize * 0.85, fontWeight: 700, color: hl ? theme.text : theme.muted}}>{item.label}</div>
                <div
                  style={{
                    fontFamily: fonts.mono,
                    fontSize: layout.bodySize * 0.85,
                    fontWeight: 700,
                    color: hl ? theme.accent : theme.muted,
                    fontVariantNumeric: 'tabular-nums',
                  }}
                >
                  {d.prefix ?? ''}
                  {formatNumber(Number(item.value) * Math.min(grow, 1), decimals)}
                  {d.unit ?? ''}
                </div>
              </div>
              <div style={{height: 44, borderRadius: 14, background: withAlpha(theme.dark ? '#FFFFFF' : '#000000', 0.06)}}>
                <div
                  style={{
                    width: `${Math.max(w, 0.004) * 100}%`,
                    height: '100%',
                    borderRadius: 14,
                    background: hl ? `linear-gradient(90deg, ${withAlpha(theme.accent, 0.7)}, ${theme.accent})` : color,
                    boxShadow: hl ? `0 0 40px ${withAlpha(theme.accent, 0.45)}` : undefined,
                  }}
                />
              </div>
            </div>
          );
        })}
      </div>
      {d.note ? (
        <div
          style={{
            fontSize: layout.bodySize * 0.8,
            color: theme.muted,
            fontWeight: 600,
            marginTop: 8,
            ...riseStyle(spring({frame: frame - (scene.cues[items.length] ?? 999), fps, config: springs.smooth}), 20, 6),
          }}
        >
          {d.note}
        </div>
      ) : null}
    </div>
  );
};
