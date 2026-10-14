import React from 'react';
import {interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {activeCue, ArrowIcon, ChevronIcon, clamp, Kicker, Panel, riseStyle, RichText, springs, useSpringAt, withAlpha} from '../motion';
import {fonts, Theme, useStyle} from '../theme';
import type {SceneProps} from '../types';

const KEYWORDS = new Set(
  (
    'fn let mut pub struct enum impl trait use mod match if else for while loop return break continue in ' +
    'const static self Self as where async await move ref type dyn unsafe crate super ' +
    'function var new class extends import export from default this null undefined true false typeof ' +
    'def lambda pass yield with try except finally raise None True False and or not is elif ' +
    'func package interface go defer chan select range map struct'
  ).split(' '),
);

type Tok = {text: string; color: string};

const HASH_COMMENT = new Set(['python', 'py', 'bash', 'sh', 'shell', 'zsh', 'ruby', 'yaml', 'toml']);

const tokenizeLine = (line: string, theme: Theme, language: string): Tok[] => {
  const s = theme.syntax;
  const comment = HASH_COMMENT.has(language) ? /^(#.*|\/\/.*)/ : /^(\/\/.*|--.*)/;
  const out: Tok[] = [];
  let rest = line;
  while (rest.length) {
    let m: RegExpMatchArray | null;
    if ((m = rest.match(comment))) {
      out.push({text: m[0], color: s.comment});
    } else if ((m = rest.match(/^("(?:[^"\\]|\\.)*"?|'(?:[^'\\]|\\.)*'?|`[^`]*`?)/))) {
      out.push({text: m[0], color: s.string});
    } else if ((m = rest.match(/^\d[\d_.]*/))) {
      out.push({text: m[0], color: s.number});
    } else if ((m = rest.match(/^[A-Za-z_][A-Za-z0-9_]*/))) {
      const word = m[0];
      const after = rest.slice(word.length);
      let color = theme.text;
      if (KEYWORDS.has(word)) color = s.keyword;
      else if (/^\s*[(!]/.test(after)) color = s.fn;
      else if (/^[A-Z]/.test(word)) color = s.type;
      out.push({text: word, color});
    } else if ((m = rest.match(/^\s+/))) {
      out.push({text: m[0], color: theme.text});
    } else {
      out.push({text: rest[0], color: s.punct});
      m = [rest[0]] as unknown as RegExpMatchArray;
    }
    rest = rest.slice(m[0].length);
  }
  return out;
};

const WindowBar: React.FC<{title: string}> = ({title}) => {
  const {theme} = useStyle();
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        padding: '22px 28px',
        borderBottom: `2px solid ${theme.panelBorder}`,
        position: 'relative',
      }}
    >
      {['#FF5F57', '#FEBC2E', '#28C840'].map((c) => (
        <div key={c} style={{width: 20, height: 20, borderRadius: 10, background: c, marginRight: 12}} />
      ))}
      <div
        style={{
          position: 'absolute',
          left: 0,
          right: 0,
          textAlign: 'center',
          fontFamily: fonts.mono,
          fontSize: 26,
          color: theme.muted,
        }}
      >
        {title}
      </div>
    </div>
  );
};

export const Code: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const d = scene.data;
  const language = (d.language ?? '').toLowerCase();
  const lines: string[] = String(d.code).replace(/\t/g, '    ').split('\n');
  const longest = Math.max(...lines.map((l) => l.length), 16);
  const fontSize = Math.min(40, (layout.content.w - 120) / (longest * 0.6));
  const lineH = fontSize * 1.6;
  const typeStart = scene.extra.typeStart ?? 6;
  const typeEnd = scene.extra.typeEnd ?? typeStart;
  const total = lines.reduce((n, l) => n + l.length + 1, 0);
  const typed = Math.floor(interpolate(frame, [typeStart, Math.max(typeEnd, typeStart + 1)], [0, total], clamp));
  const highlights: any[] = d.highlights ?? [];
  const active = activeCue(scene.cues, frame);
  const hl = active >= 0 ? highlights[active] : null;
  const range: [number, number] | null = hl ? [hl.lines[0], hl.lines[hl.lines.length - 1]] : null;
  const prev = active > 0 ? highlights[active - 1] : null;
  const move = spring({frame: frame - (scene.cues[active] ?? 0), fps, config: springs.snappy});
  const bandTop = range
    ? interpolate(move, [0, 1], [prev ? prev.lines[0] - 1 : range[0] - 1, range[0] - 1]) * lineH
    : 0;
  const bandH = range ? (range[1] - range[0] + 1) * lineH : 0;
  const win = useSpringAt(0, springs.snappy);
  const note = hl?.note;
  const noteP = spring({frame: frame - (scene.cues[active] ?? 0) - 6, fps, config: springs.smooth});
  const blink = Math.floor(frame / 15) % 2 === 0 || typed < total;

  let budget = typed;
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 30}}>
      <Kicker text={d.kicker} />
      {d.heading ? <RichText text={d.heading} size={layout.titleSize * 0.58} weight={800} start={2} stagger={2} /> : null}
      <div style={{...riseStyle(win, 60, 12), transform: `${riseStyle(win, 60, 12).transform} scale(${0.94 + 0.06 * win})`}}>
        <Panel style={{overflow: 'hidden', background: theme.dark ? 'rgba(10, 12, 24, 0.86)' : 'rgba(255,255,255,0.92)'}}>
          <WindowBar title={d.filename ?? 'main'} />
          <div style={{position: 'relative', padding: '28px 0 32px'}}>
            {range ? (
              <div
                style={{
                  position: 'absolute',
                  left: 0,
                  right: 0,
                  top: 28 + bandTop,
                  height: bandH,
                  background: withAlpha(theme.accent, 0.12),
                  borderLeft: `6px solid ${theme.accent}`,
                  opacity: interpolate(move, [0, 0.3], [prev ? 1 : 0, 1], clamp),
                }}
              />
            ) : null}
            {lines.map((line, i) => {
              const visible = Math.max(0, Math.min(line.length, budget));
              const showCursor = budget >= 0 && budget <= line.length && typed < total;
              budget -= line.length + 1;
              const lineNo = i + 1;
              const inRange = range ? lineNo >= range[0] && lineNo <= range[1] : true;
              let left = visible;
              return (
                <div
                  key={i}
                  style={{
                    position: 'relative',
                    display: 'flex',
                    height: lineH,
                    alignItems: 'center',
                    fontFamily: fonts.mono,
                    fontSize,
                    whiteSpace: 'pre',
                    opacity: inRange ? 1 : 0.3,
                  }}
                >
                  <span style={{width: 64, textAlign: 'right', paddingRight: 22, color: theme.faint, flex: 'none', fontSize: fontSize * 0.8}}>{lineNo}</span>
                  {tokenizeLine(line, theme, language).map((tok, k) => {
                    if (left <= 0) return null;
                    const text = tok.text.slice(0, left);
                    left -= tok.text.length;
                    return (
                      <span key={k} style={{color: tok.color}}>
                        {text}
                      </span>
                    );
                  })}
                  {showCursor || (i === lines.length - 1 && typed >= total && blink && !range) ? (
                    <span
                      style={{
                        display: 'inline-block',
                        width: fontSize * 0.58,
                        height: fontSize * 1.15,
                        background: theme.accent,
                        marginLeft: 2,
                        opacity: blink ? 1 : 0,
                      }}
                    />
                  ) : null}
                </div>
              );
            })}
          </div>
        </Panel>
      </div>
      <div style={{minHeight: layout.bodySize * 1.4}}>
        {note ? (
          <div
            key={active}
            style={{
              fontSize: layout.bodySize * 0.9,
              fontWeight: 700,
              color: theme.text,
              display: 'flex',
              gap: 18,
              alignItems: 'baseline',
              ...riseStyle(noteP, 20, 6),
            }}
          >
            <ArrowIcon size={layout.bodySize * 0.9} color={theme.accent} />
            <span>{note}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
};

export const Terminal: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const cpf = scene.extra.charsPerFrame ?? 1.2;
  const win = useSpringAt(0, springs.snappy);
  const fontSize = layout.format === 'vertical' ? 34 : 32;
  const entries: any[] = d.lines;
  const blink = Math.floor(frame / 15) % 2 === 0;
  const lastStart = scene.cues[scene.cues.length - 1] ?? 0;
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 30}}>
      <Kicker text={d.kicker} />
      {d.heading ? <RichText text={d.heading} size={layout.titleSize * 0.58} weight={800} start={2} stagger={2} /> : null}
      <div style={riseStyle(win, 60, 12)}>
        <Panel style={{overflow: 'hidden', background: theme.dark ? 'rgba(6, 8, 14, 0.9)' : 'rgba(25,25,28,0.95)'}}>
          <WindowBar title={d.title ?? 'zsh'} />
          <div style={{padding: '30px 36px 40px', fontFamily: fonts.mono, fontSize, lineHeight: 1.6, minHeight: 300}}>
            {entries.map((e, i) => {
              const start = scene.cues[i] ?? i * 30;
              if (frame < start) return null;
              const cmd = String(e.cmd);
              const chars = Math.min(cmd.length, Math.floor((frame - start) * cpf));
              const doneAt = start + Math.ceil(cmd.length / cpf);
              const out: string[] = e.out ?? [];
              const typing = chars < cmd.length;
              return (
                <div key={i} style={{marginBottom: 14}}>
                  <div style={{display: 'flex', whiteSpace: 'pre-wrap'}}>
                    <span style={{marginRight: 14, display: 'flex'}}><ChevronIcon size={fontSize * 0.9} color={theme.accent} /></span>
                    <span style={{color: '#F5F6FB'}}>{cmd.slice(0, chars)}</span>
                    {typing || (start === lastStart && out.length === 0 && blink) ? (
                      <span style={{display: 'inline-block', width: fontSize * 0.58, background: theme.accent, marginLeft: 2}} />
                    ) : null}
                  </div>
                  {out.map((line, k) => {
                    const at = doneAt + 5 + k * 3;
                    const o = interpolate(frame, [at, at + 4], [0, 1], clamp);
                    return (
                      <div key={k} style={{color: '#A3A9C2', opacity: o, whiteSpace: 'pre-wrap', paddingLeft: fontSize * 1.2}}>
                        {line}
                      </div>
                    );
                  })}
                </div>
              );
            })}
          </div>
        </Panel>
      </div>
    </div>
  );
};
