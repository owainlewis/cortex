import React from 'react';
import {interpolate, useCurrentFrame} from 'remotion';
import {activeCue, badgeSlot, Card, clamp, CurvedArrow, formatNumber, Icon, Pill, progress} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';
import {Heading} from './ListScenes';

// Cards joined by hand-drawn arrows, each arriving as it is spoken.
export const Flow: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const nodes: any[] = d.nodes.map((n: any) => (typeof n === 'string' ? {label: n} : n));
  const row = layout.format !== 'vertical';
  const active = activeCue(scene.cues, frame);
  const gap = row ? Math.min(140, layout.content.w * 0.08) : 70;
  const cardW = row ? (layout.content.w - gap * (nodes.length - 1)) / nodes.length : layout.content.w;
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <div style={{display: 'flex', flexDirection: row ? 'row' : 'column', alignItems: 'center'}}>
        {nodes.map((node, i) => {
          const at = scene.cues[i] ?? i * 15;
          const p = progress(frame, at, 20);
          return (
            <React.Fragment key={i}>
              {i > 0 ? <CurvedArrow length={gap} p={progress(frame, at - 10, 12)} vertical={!row} /> : null}
              <div style={{position: 'relative', width: cardW}}>
                {node.badge ? (
                  <div style={badgeSlot(row, layout.body)}>
                    <Pill text={node.badge} tone="accent" icon="check" p={progress(frame, at + 10, 12)} size={layout.body * 0.68} />
                  </div>
                ) : null}
                <Card p={p}>
                  <div
                    style={{
                      display: 'flex',
                      flexDirection: row ? 'column' : 'row',
                      alignItems: 'center',
                      textAlign: row ? 'center' : 'left',
                      gap: row ? layout.body * 0.6 : layout.body * 0.8,
                      padding: row ? `${layout.body * 1.3}px ${layout.body * 0.8}px` : `${layout.body * 0.7}px ${layout.body * 0.9}px`,
                    }}
                  >
                    <Icon name={node.icon ?? 'circle-dot'} size={row ? layout.body * 2.2 : layout.body * 1.5} color={i <= active ? theme.accent : theme.faint} stroke={1.6} />
                    <div>
                      <div style={{fontFamily: fonts.serif, fontSize: layout.body * 1.3, lineHeight: 1.05, letterSpacing: '-0.01em'}}>{node.label}</div>
                      {node.sub ? <div style={{fontSize: layout.body * 0.7, color: theme.muted, marginTop: 10, lineHeight: 1.35}}>{node.sub}</div> : null}
                    </div>
                  </div>
                </Card>
              </div>
            </React.Fragment>
          );
        })}
      </div>
    </div>
  );
};

type TreeNode = {label: string; children?: TreeNode[]};
type Placed = {label: string; x: number; depth: number; parent: number; path: string; leaf: boolean};

// Leaves are spaced evenly, parents sit centred over their children.
const placeTree = (root: TreeNode) => {
  const out: Placed[] = [];
  let leafIndex = 0;
  const walk = (node: TreeNode, depth: number, parent: number, path: string): number => {
    const me = out.length;
    out.push({label: node.label, x: 0, depth, parent, path, leaf: !node.children?.length});
    if (!node.children?.length) {
      out[me].x = leafIndex++;
    } else {
      const xs = node.children.map((c, i) => out[walk(c, depth + 1, me, path ? `${path}.${i}` : `${i}`)].x);
      out[me].x = (xs[0] + xs[xs.length - 1]) / 2;
    }
    return me;
  };
  walk(root, 0, -1, '');
  return {nodes: out, leaves: leafIndex, depth: Math.max(...out.map((n) => n.depth))};
};

export const Tree: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const {nodes, leaves, depth} = placeTree(d.root);
  const W = layout.content.w;
  const reserved = layout.title * 2.3 + 90 + (d.highlight?.note ? layout.body * 2.4 + 40 : 0);
  const avail = layout.content.h - reserved;
  const nodeH = Math.min(84, (avail / (depth + 1)) * 0.7);
  const levelH = Math.min(180, (avail - nodeH) / Math.max(depth, 1));
  const H = depth * levelH + nodeH;
  const colW = W / Math.max(leaves, 1);
  const nodeW = Math.min(colW - 20, 300);
  const fontSize = Math.min(34, nodeW / 6.6, nodeH * 0.4);
  const cx = (n: Placed) => colW * (n.x + 0.5);
  const top = (n: Placed) => n.depth * levelH;

  const hlCue = d.highlight ? scene.cues[depth + 1] : undefined;
  const hlPath: string | null = d.highlight ? (d.highlight.path as number[]).join('.') : null;
  const hlOn = hlCue !== undefined && frame >= hlCue;
  const onPath = (n: Placed) => hlPath !== null && (n.path === '' || hlPath === n.path || hlPath.startsWith(`${n.path}.`));
  const travel = hlCue !== undefined ? interpolate(frame, [hlCue, hlCue + 10 * (depth + 1)], [0, depth + 0.999], clamp) : -1;

  return (
    <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <div style={{position: 'relative', width: W, height: H}}>
        <svg width={W} height={H} style={{position: 'absolute', inset: 0, overflow: 'visible'}}>
          {nodes.map((n, i) => {
            if (n.parent < 0) return null;
            const p = nodes[n.parent];
            const at = scene.cues[n.depth] ?? n.depth * 12;
            const draw = progress(frame, at - 10, 16);
            const x1 = cx(p);
            const y1 = top(p) + nodeH;
            const x2 = cx(n);
            const y2 = top(n);
            const my = (y1 + y2) / 2;
            const len = Math.hypot(x2 - x1, y2 - y1) * 1.2;
            const lit = hlOn && onPath(n) && travel >= n.depth - 0.5;
            return (
              <path
                key={i}
                d={`M${x1} ${y1} C ${x1} ${my}, ${x2} ${my}, ${x2} ${y2}`}
                fill="none"
                stroke={lit ? theme.accent : theme.faint}
                strokeWidth={lit ? 4 : 2.5}
                strokeLinecap="round"
                strokeDasharray={len}
                strokeDashoffset={len * (1 - draw)}
              />
            );
          })}
        </svg>
        {nodes.map((n, i) => {
          const at = scene.cues[n.depth] ?? n.depth * 12;
          const sibling = nodes.filter((m) => m.depth === n.depth).indexOf(n);
          const p = progress(frame, at + sibling * 3, 18);
          const lit = hlOn && onPath(n) && travel >= n.depth - 0.2;
          const dim = hlOn && !onPath(n);
          return (
            <div
              key={i}
              style={{
                position: 'absolute',
                left: cx(n) - nodeW / 2,
                top: top(n),
                width: nodeW,
                height: nodeH,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontFamily: n.leaf ? fonts.mono : fonts.sans,
                fontWeight: n.leaf ? 500 : 650,
                fontSize,
                whiteSpace: 'pre',
                borderRadius: 16,
                color: lit ? theme.accentInk : theme.text,
                background: lit ? theme.accent : theme.card,
                border: `1.5px solid ${lit ? theme.accent : theme.border}`,
                boxShadow: theme.shadow,
                opacity: p * (dim ? 0.4 : 1),
                transform: `translateY(${(1 - p) * 14}px) scale(${0.96 + 0.04 * p})`,
              }}
            >
              {n.leaf && d.quoteLeaves ? `"${n.label}"` : n.label}
            </div>
          );
        })}
      </div>
      {d.highlight?.note ? (
        <div style={{marginTop: 44, minHeight: layout.body * 2}}>
          <Pill text={d.highlight.note} icon="arrow-right" tone="light" p={progress(frame, (hlCue ?? 0) + 10 * (depth + 1), 14)} />
        </div>
      ) : null}
    </div>
  );
};

// A contiguous buffer taking one insert: every cell after the cursor shifts
// right, last cell first, the way memmove copies an overlapping range.
export const ArrayScene: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const chars: string[] = Array.from(String(d.text));
  const at: number = Math.min(Math.max(Number(d.insertAt ?? 0), 0), chars.length);
  const insert = String(d.insert ?? '|');
  const stagger = scene.extra.shiftStagger ?? 2;
  const shiftAt = scene.cues[0] ?? 20;
  const moved = chars.length - at;
  const dropAt = shiftAt + moved * stagger + 6;
  const pitch = Math.min(120, layout.content.w / (chars.length + 1));
  const cell = pitch * 0.88;
  const rowW = (chars.length + 1) * pitch;
  const count = Math.round(interpolate(frame, [shiftAt, dropAt], [0, moved], clamp));
  const drop = progress(frame, dropAt, 16);
  const cellStyle: React.CSSProperties = {
    position: 'absolute',
    width: cell,
    height: cell,
    borderRadius: 14,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontFamily: fonts.mono,
    fontSize: cell * 0.44,
    boxShadow: theme.shadow,
  };
  return (
    <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <div style={{position: 'relative', width: rowW, height: cell * 2.1}}>
        {chars.map((ch, i) => {
          const appear = progress(frame, 4 + i, 14);
          const order = chars.length - 1 - i;
          const shift = i >= at ? progress(frame, shiftAt + order * stagger, 12) : 0;
          const moving = shift > 0 && shift < 1;
          return (
            <div
              key={i}
              style={{
                ...cellStyle,
                left: (i + shift) * pitch,
                top: cell * 0.6 - (moving ? cell * 0.22 * Math.sin(shift * Math.PI) : 0),
                background: theme.card,
                border: `1.5px solid ${moving ? theme.accent : theme.border}`,
                color: i >= at && shift > 0 ? theme.text : theme.muted,
                opacity: appear,
              }}
            >
              {ch === ' ' ? '·' : ch}
            </div>
          );
        })}
        <div style={{...cellStyle, left: at * pitch, top: cell * 0.6 - (1 - drop) * cell * 0.9, background: theme.accent, color: theme.accentInk, fontWeight: 600, opacity: drop}}>{insert}</div>
        <div style={{position: 'absolute', left: at * pitch - (pitch - cell) / 2 - 2, top: cell * 0.4, width: 4, height: cell * 1.4, borderRadius: 2, background: theme.accent, opacity: progress(frame, 10, 10) * (1 - drop)}} />
      </div>
      <div style={{display: 'flex', alignItems: 'baseline', gap: 20, marginTop: layout.body * 0.8, opacity: progress(frame, shiftAt - 4, 14)}}>
        <span style={{fontFamily: fonts.serif, fontSize: layout.display * 0.8, lineHeight: 1, fontVariantNumeric: 'tabular-nums'}}>{formatNumber(count)}</span>
        <span style={{fontSize: layout.body, color: theme.muted, fontWeight: 600}}>{d.counterLabel ?? 'bytes moved'}</span>
      </div>
      {d.note ? (
        <div style={{marginTop: 30}}>
          <Pill text={d.note} p={progress(frame, dropAt + 8, 14)} />
        </div>
      ) : null}
    </div>
  );
};
