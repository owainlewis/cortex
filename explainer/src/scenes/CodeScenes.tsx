import React from 'react';
import {interpolate, useCurrentFrame} from 'remotion';
import {activeCue, Card, CardHeader, clamp, Pill, progress} from '../motion';
import {fonts, Theme, useStyle} from '../theme';
import type {SceneProps} from '../types';
import {Heading} from './ListScenes';

const KEYWORDS = new Set(
  (
    'fn let mut pub struct enum impl trait use mod match if else for while loop return break continue in ' +
    'const static self Self as where async await move ref type dyn unsafe crate super ' +
    'function var new class extends import export from default this null undefined true false typeof ' +
    'def lambda pass yield with try except finally raise None True False and or not is elif ' +
    'func package interface go defer chan select range map'
  ).split(' '),
);
const HASH_COMMENT = new Set(['python', 'py', 'bash', 'sh', 'shell', 'zsh', 'ruby', 'yaml', 'toml']);

type Tok = {text: string; color: string};

// Code sits on a dark card, so colours come from the dark syntax palette.
const tokenizeLine = (line: string, theme: Theme, language: string): Tok[] => {
  const s = theme.syntax;
  const base = theme.inkText;
  const comment = HASH_COMMENT.has(language) ? /^(#.*|\/\/.*)/ : /^(\/\/.*|--.*)/;
  const out: Tok[] = [];
  let rest = line;
  while (rest.length) {
    let m: RegExpMatchArray | null;
    let color = base;
    if ((m = rest.match(comment))) color = s.comment;
    else if ((m = rest.match(/^("(?:[^"\\]|\\.)*"?|'(?:[^'\\]|\\.)*'?|`[^`]*`?)/))) color = s.string;
    else if ((m = rest.match(/^\d[\d_.]*/))) color = s.number;
    else if ((m = rest.match(/^[A-Za-z_][A-Za-z0-9_]*/))) color = KEYWORDS.has(m[0]) ? s.keyword : base;
    else if ((m = rest.match(/^\s+/))) color = base;
    else {
      m = [rest[0]] as unknown as RegExpMatchArray;
      color = theme.inkMuted;
    }
    out.push({text: m[0], color});
    rest = rest.slice(m[0].length);
  }
  return out;
};

export const Code: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const language = (d.language ?? '').toLowerCase();
  const lines: string[] = String(d.code).replace(/\t/g, '    ').split('\n');
  const longest = Math.max(...lines.map((l) => l.length), 18);
  const headerH = (d.kicker ? layout.small + 40 : 0) + (d.heading ? layout.title * 2.2 + 28 : 0);
  const noteH = layout.body * 2 + 40;
  const fontSize = Math.min(36, (layout.content.w - 140) / (longest * 0.6), (layout.content.h - headerH - noteH - 140) / (lines.length * 1.6));
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
  const move = progress(frame, scene.cues[active] ?? 0, 14);
  const bandTop = range ? interpolate(move, [0, 1], [prev ? prev.lines[0] - 1 : range[0] - 1, range[0] - 1]) * lineH : 0;
  const bandH = range ? (range[1] - range[0] + 1) * lineH : 0;
  const caretOn = Math.floor(frame / 15) % 2 === 0;
  let budget = typed;
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <Card dark stacked={false} p={progress(frame, 0, 20)}>
        <CardHeader dark icon="file-code" title={d.filename ?? 'code'} right={<span style={{fontFamily: fonts.mono, fontSize: layout.small, color: theme.inkMuted}}>{language}</span>} />
        <div style={{position: 'relative', padding: '24px 0 28px'}}>
          {range ? (
            <div style={{position: 'absolute', left: 0, right: 0, top: 24 + bandTop, height: bandH, borderLeft: `5px solid ${theme.accent}`, background: 'rgba(255,255,255,0.06)'}} />
          ) : null}
          {lines.map((line, i) => {
            const visible = Math.max(0, Math.min(line.length, budget));
            const caret = budget >= 0 && budget <= line.length && typed < total;
            budget -= line.length + 1;
            const lineNo = i + 1;
            const inRange = range ? lineNo >= range[0] && lineNo <= range[1] : true;
            let left = visible;
            return (
              <div key={i} style={{position: 'relative', display: 'flex', height: lineH, alignItems: 'center', fontFamily: fonts.mono, fontSize, whiteSpace: 'pre', opacity: inRange ? 1 : 0.35}}>
                <span style={{width: 70, textAlign: 'right', paddingRight: 26, color: theme.inkMuted, flex: 'none', fontSize: fontSize * 0.75}}>{lineNo}</span>
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
                {caret ? <span style={{display: 'inline-block', width: 3, height: fontSize * 1.1, background: theme.accent, opacity: caretOn ? 1 : 0}} /> : null}
              </div>
            );
          })}
        </div>
      </Card>
      <div style={{minHeight: noteH, marginTop: 30, display: 'flex', justifyContent: 'center'}}>
        {hl?.note ? <Pill key={active} text={hl.note} icon="arrow-right" tone="light" p={progress(frame, (scene.cues[active] ?? 0) + 4, 14)} /> : null}
      </div>
    </div>
  );
};

export const Terminal: React.FC<SceneProps> = ({scene}) => {
  const {layout, theme} = useStyle();
  const frame = useCurrentFrame();
  const d = scene.data;
  const cpf = scene.extra.charsPerFrame ?? 1.2;
  const fontSize = layout.format === 'vertical' ? 30 : 30;
  const entries: any[] = d.lines;
  const caretOn = Math.floor(frame / 15) % 2 === 0;
  return (
    <div style={{display: 'flex', flexDirection: 'column'}}>
      <Heading kicker={d.kicker} text={d.heading} />
      <Card dark stacked={false} p={progress(frame, 0, 20)}>
        <CardHeader dark icon="terminal" title={d.title ?? 'Terminal'} />
        <div style={{padding: '26px 34px 34px', fontFamily: fonts.mono, fontSize, lineHeight: 1.6, minHeight: 300}}>
          {entries.map((e, i) => {
            const start = scene.cues[i] ?? i * 30;
            if (frame < start) return null;
            const cmd = String(e.cmd);
            const chars = Math.min(cmd.length, Math.floor((frame - start) * cpf));
            const doneAt = start + Math.ceil(cmd.length / cpf);
            const out: string[] = e.out ?? [];
            const next = scene.cues[i + 1] ?? Infinity;
            const caret = frame < next && (chars < cmd.length || caretOn);
            return (
              <div key={i} style={{marginBottom: 16}}>
                <div style={{whiteSpace: 'pre-wrap'}}>
                  <span style={{color: theme.syntax.keyword}}>$ </span>
                  <span style={{color: theme.inkText}}>{cmd.slice(0, chars)}</span>
                  {caret ? <span style={{display: 'inline-block', width: fontSize * 0.55, height: fontSize * 1.1, verticalAlign: 'text-bottom', background: theme.syntax.keyword, marginLeft: 2}} /> : null}
                </div>
                {out.map((line, k) => {
                  const at = doneAt + 5 + k * 3;
                  return (
                    <div key={k} style={{color: theme.inkMuted, opacity: interpolate(frame, [at, at + 4], [0, 1], clamp), whiteSpace: 'pre-wrap', minHeight: '1.6em'}}>
                      {line}
                    </div>
                  );
                })}
              </div>
            );
          })}
        </div>
      </Card>
    </div>
  );
};
