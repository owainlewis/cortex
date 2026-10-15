import React from 'react';
import {useCurrentFrame} from 'remotion';
import {badgeSlot, Card, CardHeader, formatNumber, Icon, Label, Pill, progress, RichText, Strike} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

type Item = {text: string; icon?: string; detail?: string; badge?: string};
const asItem = (item: any): Item => (typeof item === 'string' ? {text: item} : {...item, text: item.text ?? item.label});

export const Heading: React.FC<{kicker?: string; text?: string}> = ({kicker, text}) => {
  const {layout} = useStyle();
  if (!kicker && !text) return null;
  return (
    <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 22, marginBottom: layout.body * 1.4, textAlign: 'center'}}>
      <Label text={kicker} align="center" />
      {text ? <RichText text={text} serif size={layout.title} start={2} align="center" /> : null}
    </div>
  );
};

// One option card: icon tile, title, optional detail line.
const OptionCard: React.FC<{item: Item; p: number; row: boolean; index: number; faded?: boolean; strike?: number; height?: number}> = ({item, p, row, index, faded, strike = 0, height}) => {
  const {layout, theme} = useStyle();
  const tile = layout.body * 2.1;
  const iconTile = (
    <div style={{width: tile, height: tile, borderRadius: 16, background: theme.accentSoft, display: 'flex', alignItems: 'center', justifyContent: 'center', flex: 'none'}}>
      {item.icon ? <Icon name={item.icon} size={tile * 0.5} /> : <span style={{fontFamily: fonts.serif, fontSize: tile * 0.55, color: theme.accent}}>{index + 1}</span>}
    </div>
  );
  return (
    <div style={{position: 'relative', flex: row ? 1 : undefined}}>
      {item.badge ? (
        <div style={badgeSlot(row, layout.body)}>
          <Pill text={item.badge} tone="accent" icon="check" p={progress(p * 30, 18, 10)} size={layout.body * 0.7} />
        </div>
      ) : null}
      <Card p={p} style={{height, opacity: p * (faded ? 0.55 : 1)}}>
        <div
          style={{
            display: 'flex',
            flexDirection: row ? 'column' : 'row',
            alignItems: row ? 'center' : 'center',
            textAlign: row ? 'center' : 'left',
            gap: layout.body * 0.7,
            padding: layout.body * (row ? 1.1 : 0.75),
            height: '100%',
            boxSizing: 'border-box',
            justifyContent: row ? 'center' : 'flex-start',
          }}
        >
          {iconTile}
          <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
            <div style={{fontFamily: row ? fonts.serif : fonts.sans, fontSize: row ? layout.body * 1.25 : layout.body, fontWeight: row ? 400 : 650, letterSpacing: row ? '-0.01em' : '-0.02em', lineHeight: 1.12, color: faded ? theme.muted : theme.text}}>
              {item.text}
            </div>
            {item.detail ? <div style={{fontSize: layout.body * 0.72, color: theme.muted, lineHeight: 1.35}}>{item.detail}</div> : null}
          </div>
        </div>
        {strike > 0 ? <Strike p={strike} /> : null}
      </Card>
    </div>
  );
};

export const List: React.FC<SceneProps> = ({scene}) => {
  const {layout} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const items = (d.items as any[]).map(asItem);
  const row = layout.format === 'landscape' && items.length <= 4;
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <div style={{display: 'flex', flexDirection: row ? 'row' : 'column', gap: row ? 48 : 30, alignItems: 'stretch'}}>
        {items.map((item, i) => (
          <OptionCard key={i} item={item} index={i} row={row} p={progress(frame, scene.cues[i], 20)} height={row ? layout.content.h * 0.5 : undefined} />
        ))}
      </div>
    </div>
  );
};

// The rejected side is shown, then struck through as the better answer lands.
export const Compare: React.FC<SceneProps> = ({scene}) => {
  const {layout} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const left = (d.left.items as any[]).map(asItem);
  const right = (d.right.items as any[]).map(asItem);
  const leftAt = scene.cues[0] ?? 6;
  const rightAt = scene.cues[1] ?? leftAt + 30;
  const row = layout.format === 'landscape';
  const pill = progress(frame, rightAt + 6, 14);
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <div style={{display: 'flex', flexDirection: row ? 'row' : 'column', gap: row ? 44 : 26, alignItems: 'stretch'}}>
        {left.map((item, i) => (
          <OptionCard
            key={`l${i}`}
            item={item}
            index={i}
            row={row}
            faded={frame >= rightAt}
            strike={progress(frame, rightAt + i * 5, 14)}
            p={progress(frame, leftAt + i * 5, 20)}
            height={row ? layout.content.h * 0.48 : undefined}
          />
        ))}
        {right.map((item, i) => (
          <div key={`r${i}`} style={{position: 'relative', flex: row ? 1 : undefined}}>
            {i === 0 && d.right.label ? (
              <div style={badgeSlot(row, layout.body)}>
                <Pill text={d.right.label} p={pill} size={layout.body * 0.72} />
              </div>
            ) : null}
            <OptionCard item={{...item, icon: item.icon}} index={left.length + i} row={row} p={progress(frame, rightAt + i * 5, 20)} height={row ? layout.content.h * 0.48 : undefined} />
          </div>
        ))}
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
  const card = progress(frame, 0, 20);
  const chartH = layout.content.h * (layout.format === 'vertical' ? 0.5 : 0.48);
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Label text={d.kicker} align="center" style={{marginBottom: 28}} />
      <Card p={card}>
        <CardHeader icon={d.icon ?? 'chart-column'} title={d.heading ? d.heading.replace(/[*=]/g, '') : 'Comparison'} />
        <div style={{display: 'flex', alignItems: 'flex-end', justifyContent: 'center', gap: layout.body * 1.2, padding: `${layout.body}px ${layout.body * 1.4}px ${layout.body * 0.8}px`, height: chartH}}>
          {items.map((item, i) => {
            const at = scene.cues[i] ?? i * 8;
            const grow = progress(frame, at, 26);
            const hl = !!item.highlight;
            const h = Math.max((Number(item.value) / max) * grow, grow > 0 ? 0.012 : 0);
            return (
              <div key={i} style={{flex: 1, maxWidth: layout.body * 4, height: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'flex-end', gap: 14}}>
                <div style={{fontFamily: fonts.mono, fontSize: layout.body * 0.62, color: hl ? theme.accent : theme.muted, opacity: progress(frame, at + 4, 12), whiteSpace: 'nowrap'}}>
                  {d.prefix ?? ''}
                  {formatNumber(Number(item.value) * grow, decimals)}
                  {d.unit ?? ''}
                </div>
                <div style={{width: '100%', height: `${h * 78}%`, borderRadius: 10, background: hl ? theme.accent : theme.border}} />
                <div style={{fontSize: layout.body * 0.7, fontWeight: 600, color: hl ? theme.text : theme.muted, textAlign: 'center', opacity: progress(frame, at, 12)}}>{item.label}</div>
              </div>
            );
          })}
        </div>
      </Card>
      {d.note ? (
        <div style={{display: 'flex', justifyContent: 'center', marginTop: 32}}>
          <Pill text={d.note} p={progress(frame, scene.cues[items.length] ?? 999, 14)} />
        </div>
      ) : null}
    </div>
  );
};
