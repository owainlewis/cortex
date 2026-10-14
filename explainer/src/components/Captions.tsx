import React, {useMemo} from 'react';
import {useCurrentFrame, useVideoConfig} from 'remotion';
import {progress} from '../motion';
import {fonts, useStyle} from '../theme';
import type {SceneTiming, Word} from '../types';

type Page = {words: Word[]; start: number; end: number};

const MAX_WORDS = 4;
const MAX_CHARS = 20;

// Split narration into short caption pages. Pages break on sentence and
// clause punctuation, on pauses, and when they get too long to read at a
// glance.
const paginate = (words: Word[], fps: number): Page[] => {
  const pages: Page[] = [];
  let cur: Word[] = [];
  const flush = () => {
    if (cur.length) pages.push({words: cur, start: cur[0].start, end: cur[cur.length - 1].end});
    cur = [];
  };
  words.forEach((w, i) => {
    const prev = cur[cur.length - 1];
    const chars = cur.reduce((n, x) => n + x.text.length + 1, 0) + w.text.length;
    if (prev && (w.start - prev.end > fps * 0.35 || cur.length >= MAX_WORDS || chars > MAX_CHARS)) flush();
    cur.push(w);
    const next = words[i + 1];
    if (/[.!?;:,]$/.test(w.text) || !next) flush();
  });
  return pages;
};

export const Captions: React.FC<{scenes: SceneTiming[]}> = ({scenes}) => {
  const frame = useCurrentFrame();
  const {fps, width} = useVideoConfig();
  const {theme, layout} = useStyle();

  // Only caption scenes that ask for it (hook and outro text already carries
  // the narration on screen).
  const visibleWords = useMemo(() => {
    const out: Word[] = [];
    for (const s of scenes) {
      if (!s.captions) continue;
      for (const w of s.words) out.push({...w, start: w.start + s.from, end: w.end + s.from});
    }
    return out;
  }, [scenes]);
  const pages = useMemo(() => paginate(visibleWords, fps), [visibleWords, fps]);

  const index = pages.findIndex((p, i) => {
    const next = pages[i + 1];
    const hold = next ? Math.min(next.start, p.end + fps * 0.5) : p.end + fps * 0.5;
    return frame >= p.start - 2 && frame < hold;
  });
  if (index < 0) return null;
  const page = pages[index];
  const enter = progress(frame, page.start - 2, 8);

  return (
    <div
      style={{
        position: 'absolute',
        left: layout.margin,
        width: width - layout.margin * 2,
        top: layout.captionY,
        display: 'flex',
        justifyContent: 'center',
        flexWrap: 'wrap',
        columnGap: '0.26em',
        fontFamily: fonts.sans,
        fontSize: layout.captionSize,
        fontWeight: 700,
        letterSpacing: '-0.025em',
        lineHeight: 1.15,
        textAlign: 'center',
        opacity: enter,
        transform: `translateY(${(1 - enter) * 12}px)`,
      }}
    >
      {page.words.map((w, i) => {
        const spoken = frame >= w.start;
        const next = page.words[i + 1];
        const active = spoken && (!next || frame < next.start);
        return (
          <span key={i} style={{color: active ? theme.accent : spoken ? theme.text : theme.faint}}>
            {w.text}
          </span>
        );
      })}
    </div>
  );
};
