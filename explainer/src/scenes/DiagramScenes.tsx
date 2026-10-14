import React from 'react';
import {interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {activeCue, ArrowIcon, clamp, Kicker, Panel, riseStyle, RichText, springs, withAlpha} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneProps} from '../types';

export const Flow: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const d = scene.data;
  const nodes: any[] = d.nodes;
  const vertical = layout.format !== 'landscape';
  const active = activeCue(scene.cues, frame);
  const gap = vertical ? Math.max(56, 110 - nodes.length * 10) : 90;

  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 30}}>
      <Kicker text={d.kicker} />
      {d.heading ? <RichText text={d.heading} size={layout.titleSize * 0.6} weight={800} start={2} stagger={2} /> : null}
      <div
        style={{
          display: 'flex',
          flexDirection: vertical ? 'column' : 'row',
          alignItems: 'stretch',
          marginTop: 16,
        }}
      >
        {nodes.map((node, i) => {
          const at = scene.cues[i] ?? i * 15;
          const p = spring({frame: frame - at, fps, config: springs.bouncy});
          const draw = interpolate(frame, [at - 10, at], [0, 1], clamp);
          const isActive = i === active;
          const label = typeof node === 'string' ? node : node.label;
          const sub = typeof node === 'string' ? null : node.sub;
          // A pulse travels along each finished connector to show direction.
          const pulse = ((frame - at) % 36) / 36;
          return (
            <React.Fragment key={i}>
              {i > 0 ? (
                <div
                  style={{
                    position: 'relative',
                    flex: 'none',
                    width: vertical ? '100%' : gap,
                    height: vertical ? gap : 'auto',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                >
                  <svg
                    width={vertical ? 40 : gap}
                    height={vertical ? gap : 40}
                    viewBox={vertical ? `0 0 40 ${gap}` : `0 0 ${gap} 40`}
                    style={{overflow: 'visible'}}
                  >
                    <defs>
                      <marker id={`arrow-${i}`} viewBox="0 0 10 10" refX="7" refY="5" markerWidth="5" markerHeight="5" orient="auto">
                        <path d="M0 0L10 5L0 10z" fill={theme.accent} />
                      </marker>
                    </defs>
                    <line
                      x1={vertical ? 20 : 4}
                      y1={vertical ? 4 : 20}
                      x2={vertical ? 20 : 4 + (gap - 14) * draw}
                      y2={vertical ? 4 + (gap - 14) * draw : 20}
                      stroke={theme.accent}
                      strokeWidth={5}
                      strokeLinecap="round"
                      markerEnd={draw > 0.95 ? `url(#arrow-${i})` : undefined}
                      opacity={draw > 0 ? 1 : 0}
                    />
                    {draw >= 1 ? (
                      <circle
                        cx={vertical ? 20 : 4 + (gap - 14) * pulse}
                        cy={vertical ? 4 + (gap - 14) * pulse : 20}
                        r={8}
                        fill={theme.dark ? '#FFFFFF' : theme.accent}
                        opacity={0.9 * Math.sin(pulse * Math.PI)}
                      />
                    ) : null}
                  </svg>
                </div>
              ) : null}
              <div
                style={{
                  flex: vertical ? 'none' : 1,
                  opacity: interpolate(p, [0, 0.4], [0, 1], clamp),
                  transform: `scale(${interpolate(p, [0, 1], [0.7, 1])})`,
                }}
              >
                <Panel
                  glow={isActive ? 1 : 0}
                  style={{
                    padding: vertical ? '30px 36px' : '34px 30px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 26,
                    height: '100%',
                    boxSizing: 'border-box',
                    opacity: active > i ? 0.62 : 1,
                  }}
                >
                  <div
                    style={{
                      flex: 'none',
                      width: 60,
                      height: 60,
                      borderRadius: 30,
                      border: `3px solid ${theme.accent}`,
                      background: isActive ? theme.accent : 'transparent',
                      color: isActive ? theme.accentInk : theme.accent,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      fontFamily: fonts.mono,
                      fontWeight: 700,
                      fontSize: 28,
                    }}
                  >
                    {i + 1}
                  </div>
                  <div>
                    <div style={{fontSize: layout.bodySize, fontWeight: 800, lineHeight: 1.15, letterSpacing: '-0.02em'}}>{label}</div>
                    {sub ? (
                      <div style={{fontSize: layout.bodySize * 0.66, fontWeight: 600, color: theme.muted, marginTop: 6, lineHeight: 1.3}}>
                        {sub}
                      </div>
                    ) : null}
                  </div>
                </Panel>
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
  const depth = Math.max(...out.map((n) => n.depth));
  return {nodes: out, leaves: leafIndex, depth};
};

export const Tree: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const d = scene.data;
  const {nodes, leaves, depth} = placeTree(d.root);
  const W = layout.content.w;
  const levelH = layout.format === 'vertical' ? 190 : 150;
  const H = depth * levelH + 110;
  const colW = W / Math.max(leaves, 1);
  const nodeW = Math.min(colW - 18, 300);
  const nodeH = 92;
  const fontSize = Math.min(38, nodeW / 6.2);
  const cx = (n: Placed) => colW * (n.x + 0.5);
  const cy = (n: Placed) => n.depth * levelH + nodeH / 2;

  const hlCue = d.highlight ? scene.cues[depth + 1] : undefined;
  const hlPath: string | null = d.highlight ? (d.highlight.path as number[]).join('.') : null;
  const hlOn = hlCue !== undefined && frame >= hlCue;
  const onPath = (n: Placed) =>
    hlPath !== null && (n.path === '' || hlPath === n.path || hlPath.startsWith(`${n.path}.`));
  const travel = hlCue !== undefined ? interpolate(frame, [hlCue, hlCue + 12 * (depth + 1)], [0, depth + 0.999], clamp) : -1;
  const noteP = spring({frame: frame - (hlCue ?? 0) - 12 * (depth + 1), fps, config: springs.smooth});

  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 30}}>
      <Kicker text={d.kicker} />
      {d.heading ? <RichText text={d.heading} size={layout.titleSize * 0.6} weight={800} start={2} stagger={2} /> : null}
      <div style={{position: 'relative', width: W, height: H, marginTop: 24}}>
        <svg width={W} height={H} style={{position: 'absolute', inset: 0, overflow: 'visible'}}>
          {nodes.map((n, i) => {
            if (n.parent < 0) return null;
            const p = nodes[n.parent];
            const at = scene.cues[n.depth] ?? n.depth * 12;
            const draw = interpolate(frame, [at - 8, at + 4], [0, 1], clamp);
            const x1 = cx(p);
            const y1 = cy(p) + nodeH / 2;
            const x2 = cx(n);
            const y2 = cy(n) - nodeH / 2;
            const midY = (y1 + y2) / 2;
            const path = `M${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y2}`;
            const len = Math.hypot(x2 - x1, y2 - y1) * 1.25;
            const lit = hlOn && onPath(n) && travel >= n.depth - 0.5;
            return (
              <path
                key={i}
                d={path}
                fill="none"
                stroke={lit ? theme.accent : withAlpha(theme.dark ? '#FFFFFF' : '#000000', 0.25)}
                strokeWidth={lit ? 6 : 3}
                strokeDasharray={len}
                strokeDashoffset={len * (1 - draw)}
                strokeLinecap="round"
              />
            );
          })}
        </svg>
        {nodes.map((n, i) => {
          const at = scene.cues[n.depth] ?? n.depth * 12;
          const sibling = nodes.filter((m) => m.depth === n.depth).indexOf(n);
          const p = spring({frame: frame - at - sibling * 2, fps, config: springs.bouncy});
          const lit = hlOn && onPath(n) && travel >= n.depth - 0.2;
          const dim = hlOn && !onPath(n);
          return (
            <div
              key={i}
              style={{
                position: 'absolute',
                left: cx(n) - nodeW / 2,
                top: cy(n) - nodeH / 2,
                width: nodeW,
                height: nodeH,
                borderRadius: 22,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontFamily: n.leaf ? fonts.mono : fonts.display,
                fontSize: n.leaf ? fontSize * 0.92 : fontSize,
                fontWeight: n.leaf ? 600 : 800,
                whiteSpace: 'pre',
                color: lit ? theme.accentInk : n.leaf ? theme.text : theme.accent,
                background: lit
                  ? theme.accent
                  : n.leaf
                    ? theme.panel
                    : withAlpha(theme.accent, theme.dark ? 0.1 : 0.12),
                border: `2px solid ${lit ? theme.accent : n.leaf ? theme.panelBorder : withAlpha(theme.accent, 0.5)}`,
                boxShadow: lit ? `0 0 50px ${withAlpha(theme.accent, 0.55)}` : '0 16px 40px rgba(0,0,0,0.25)',
                opacity: interpolate(p, [0, 0.4], [0, dim ? 0.35 : 1], clamp),
                transform: `scale(${interpolate(p, [0, 1], [0.5, 1])})`,
              }}
            >
              {n.leaf && d.quoteLeaves ? `"${n.label}"` : n.label}
            </div>
          );
        })}
      </div>
      <div style={{minHeight: layout.bodySize * 1.4}}>
        {d.highlight?.note && hlOn ? (
          <div style={{fontSize: layout.bodySize * 0.9, fontWeight: 700, display: 'flex', gap: 18, ...riseStyle(noteP, 20, 6)}}>
            <ArrowIcon size={layout.bodySize * 0.9} color={theme.accent} />
            <span>{d.highlight.note}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
};
