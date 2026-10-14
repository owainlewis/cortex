import React from 'react';
import {interpolate, useCurrentFrame} from 'remotion';
import {activeCue, ArrowIcon, clamp, formatNumber, Label, lift, progress, RichText} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

const Header: React.FC<{kicker?: string; heading?: string}> = ({kicker, heading}) => {
  const {layout} = useStyle();
  if (!kicker && !heading) return null;
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 28, marginBottom: 56}}>
      <Label text={kicker} />
      {heading ? <RichText text={heading} size={layout.title} weight={650} start={2} lineHeight={1.06} /> : null}
    </div>
  );
};

// A numbered timeline: the spine draws down to each step as it is spoken.
export const Flow: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const nodes: any[] = d.nodes;
  const active = activeCue(scene.cues, frame);
  const dot = 52;
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Header kicker={d.kicker} heading={d.heading} />
      {nodes.map((node, i) => {
        const at = scene.cues[i] ?? i * 15;
        const p = progress(frame, at, 16);
        const nextAt = scene.cues[i + 1];
        const spine = nextAt === undefined ? 0 : progress(frame, nextAt - 12, 12);
        const isActive = i === active;
        const label = typeof node === 'string' ? node : node.label;
        const sub = typeof node === 'string' ? null : node.sub;
        return (
          <div key={i} style={{position: 'relative', display: 'flex', gap: 40, paddingBottom: i < nodes.length - 1 ? 64 : 0}}>
            {i < nodes.length - 1 ? (
              <div
                style={{
                  position: 'absolute',
                  left: dot / 2 - 1,
                  top: dot + 10,
                  bottom: 10,
                  width: 2,
                  background: theme.text,
                  transform: `scaleY(${spine})`,
                  transformOrigin: 'top',
                }}
              />
            ) : null}
            <div
              style={{
                flex: 'none',
                width: dot,
                height: dot,
                borderRadius: dot / 2,
                border: `2px solid ${isActive ? theme.accent : theme.text}`,
                background: isActive ? theme.accent : 'transparent',
                color: isActive ? theme.accentInk : theme.text,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontFamily: fonts.mono,
                fontSize: layout.small,
                opacity: p,
                transform: `scale(${0.6 + 0.4 * p})`,
              }}
            >
              {i + 1}
            </div>
            <div style={{paddingTop: 2, opacity: active > i ? 0.45 : 1}}>
              <RichText text={label} size={layout.body * 1.08} weight={600} lineHeight={1.12} letterSpacing="-0.03em" start={at + 2} stagger={1} />
              {sub ? (
                <div style={{fontSize: layout.body * 0.78, color: theme.muted, marginTop: 10, lineHeight: 1.3, ...lift(progress(frame, at + 6, 16), 10)}}>{sub}</div>
              ) : null}
            </div>
          </div>
        );
      })}
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
  const nodeH = 84;
  const levelH = Math.min(layout.format === 'vertical' ? 190 : 150, (layout.content.h - (d.highlight ? 470 : 360) - nodeH) / Math.max(depth, 1));
  const H = depth * levelH + nodeH;
  const colW = W / Math.max(leaves, 1);
  const nodeW = Math.min(colW - 16, 280);
  const fontSize = Math.min(34, nodeW / 6.4);
  const cx = (n: Placed) => colW * (n.x + 0.5);
  const top = (n: Placed) => n.depth * levelH;

  const hlCue = d.highlight ? scene.cues[depth + 1] : undefined;
  const hlPath: string | null = d.highlight ? (d.highlight.path as number[]).join('.') : null;
  const hlOn = hlCue !== undefined && frame >= hlCue;
  const onPath = (n: Placed) => hlPath !== null && (n.path === '' || hlPath === n.path || hlPath.startsWith(`${n.path}.`));
  const travel = hlCue !== undefined ? interpolate(frame, [hlCue, hlCue + 10 * (depth + 1)], [0, depth + 0.999], clamp) : -1;
  const noteP = progress(frame, (hlCue ?? 0) + 10 * (depth + 1), 16);

  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Header kicker={d.kicker} heading={d.heading} />
      <div style={{position: 'relative', width: W, height: H}}>
        <svg width={W} height={H} style={{position: 'absolute', inset: 0, overflow: 'visible'}}>
          {nodes.map((n, i) => {
            if (n.parent < 0) return null;
            const p = nodes[n.parent];
            const at = scene.cues[n.depth] ?? n.depth * 12;
            const draw = progress(frame, at - 10, 14);
            const x1 = cx(p);
            const y1 = top(p) + nodeH;
            const x2 = cx(n);
            const y2 = top(n);
            const midY = (y1 + y2) / 2;
            const len = Math.abs(midY - y1) + Math.abs(x2 - x1) + Math.abs(y2 - midY);
            const lit = hlOn && onPath(n) && travel >= n.depth - 0.5;
            return (
              <path
                key={i}
                d={`M${x1} ${y1} V${midY} H${x2} V${y2}`}
                fill="none"
                stroke={lit ? theme.accent : theme.muted}
                strokeWidth={lit ? 4 : 2}
                strokeDasharray={len}
                strokeDashoffset={len * (1 - draw)}
              />
            );
          })}
        </svg>
        {nodes.map((n, i) => {
          const at = scene.cues[n.depth] ?? n.depth * 12;
          const sibling = nodes.filter((m) => m.depth === n.depth).indexOf(n);
          const p = progress(frame, at + sibling * 2, 16);
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
                fontFamily: fonts.mono,
                fontSize,
                whiteSpace: 'pre',
                color: lit ? theme.accentInk : theme.text,
                background: lit ? theme.accent : n.leaf ? theme.surface : theme.bg,
                border: `2px solid ${lit ? theme.accent : n.leaf ? theme.surface : theme.text}`,
                opacity: p * (dim ? 0.35 : 1),
                transform: `translateY(${(1 - p) * 16}px)`,
              }}
            >
              {n.leaf && d.quoteLeaves ? `"${n.label}"` : n.label}
            </div>
          );
        })}
      </div>
      <div style={{minHeight: d.highlight?.note ? layout.body * 2.6 : 0, marginTop: d.highlight?.note ? 48 : 0}}>
        {d.highlight?.note && hlOn ? (
          <div style={{display: 'flex', gap: 20, alignItems: 'flex-start', fontSize: layout.body, fontWeight: 550, letterSpacing: '-0.02em', lineHeight: 1.2, ...lift(noteP, 14)}}>
            <div style={{paddingTop: layout.body * 0.12}}>
              <ArrowIcon size={layout.body * 0.9} color={theme.accent} />
            </div>
            <span>{d.highlight.note}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
};

// A contiguous buffer receiving one insert: every cell after the cursor
// shifts right, last cell first, the way memmove copies an overlapping range.
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
  const cell = Math.min(120, layout.content.w / (chars.length + 1));
  const gap = Math.max(4, cell * 0.06);
  const pitch = cell;
  const rowW = (chars.length + 1) * pitch;
  const count = Math.round(interpolate(frame, [shiftAt, shiftAt + moved * stagger + 6], [0, moved], clamp));
  const drop = progress(frame, dropAt, 14);
  const cursorX = at * pitch - gap / 2;
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Header kicker={d.kicker} heading={d.heading} />
      <div style={{position: 'relative', width: rowW, height: cell * 2.2}}>
        {chars.map((ch, i) => {
          const appear = progress(frame, 4 + i, 12);
          const order = chars.length - 1 - i; // last cell moves first
          const shift = i >= at ? progress(frame, shiftAt + order * stagger, 10) : 0;
          const moving = shift > 0 && shift < 1;
          return (
            <div
              key={i}
              style={{
                position: 'absolute',
                left: (i + shift) * pitch,
                top: cell * 0.6 - (moving ? cell * 0.18 * Math.sin(shift * Math.PI) : 0),
                width: cell - gap,
                height: cell - gap,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontFamily: fonts.mono,
                fontSize: cell * 0.48,
                color: i >= at && shift > 0 ? theme.text : theme.muted,
                background: theme.surface,
                border: `2px solid ${moving ? theme.accent : theme.surface}`,
                opacity: appear,
              }}
            >
              {ch === ' ' ? '·' : ch}
            </div>
          );
        })}
        <div
          style={{
            position: 'absolute',
            left: at * pitch,
            top: cell * 0.6 - (1 - drop) * cell * 0.9,
            width: cell - gap,
            height: cell - gap,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontFamily: fonts.mono,
            fontSize: cell * 0.48,
            fontWeight: 600,
            background: theme.accent,
            color: theme.accentInk,
            opacity: drop,
          }}
        >
          {insert}
        </div>
        <div style={{position: 'absolute', left: cursorX, top: cell * 0.35, width: 4, height: cell * 1.4, background: theme.accent, opacity: progress(frame, 10, 10) * (1 - drop)}} />
        {chars.map((_, i) => (
          <div
            key={`i${i}`}
            style={{
              position: 'absolute',
              left: i * pitch,
              top: cell * 1.7,
              width: cell - gap,
              textAlign: 'center',
              fontFamily: fonts.mono,
              fontSize: Math.min(22, cell * 0.24),
              color: theme.faint,
              opacity: progress(frame, 8 + i, 12),
            }}
          >
            {i}
          </div>
        ))}
      </div>
      <div style={{display: 'flex', alignItems: 'baseline', gap: 24, marginTop: 56, ...lift(progress(frame, shiftAt - 4, 14), 14)}}>
        <div style={{fontSize: layout.display * 0.9, fontWeight: 700, letterSpacing: '-0.05em', fontVariantNumeric: 'tabular-nums', color: theme.text}}>
          {formatNumber(count)}
        </div>
        <div style={{fontFamily: fonts.serif, fontStyle: 'italic', fontSize: layout.title * 0.9, color: theme.accent}}>{d.counterLabel ?? 'bytes moved'}</div>
      </div>
      {d.note ? (
        <div style={{fontSize: layout.body, color: theme.muted, lineHeight: 1.3, marginTop: 24, ...lift(progress(frame, dropAt + 8, 16), 14)}}>{d.note}</div>
      ) : null}
    </div>
  );
};
