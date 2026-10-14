import React, {useMemo} from 'react';
import {interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {springs} from '../motion';
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
  const enter = spring({frame: frame - (page.start - 2), fps, config: springs.bouncy});

  return (
    <div
      style={{
        position: 'absolute',
        left: 60,
        width: width - 120,
        top: layout.captionY,
        display: 'flex',
        justifyContent: 'center',
        flexWrap: 'wrap',
        columnGap: '0.28em',
        fontFamily: fonts.display,
        fontSize: layout.captionSize,
        fontWeight: 900,
        letterSpacing: '-0.02em',
        lineHeight: 1.15,
        textAlign: 'center',
        transform: `translateY(${(1 - enter) * 26}px) scale(${interpolate(enter, [0, 1], [0.88, 1])})`,
        opacity: interpolate(enter, [0, 0.4], [0, 1], {extrapolateRight: 'clamp'}),
      }}
    >
      {page.words.map((w, i) => {
        const active = frame >= w.start && (frame < w.end + 2 || i === page.words.length - 1);
        const spoken = frame >= w.start;
        const pop = spring({frame: frame - w.start, fps, config: springs.bouncy});
        return (
          <span
            key={i}
            style={{
              display: 'inline-block',
              color: active ? theme.accent : theme.dark ? '#FFFFFF' : theme.text,
              opacity: spoken ? 1 : 0.55,
              transform: `scale(${active ? interpolate(pop, [0, 1], [1, 1.08]) : 1})`,
              textShadow: theme.dark
                ? '0 4px 0 rgba(0,0,0,0.55), 0 0 28px rgba(0,0,0,0.85)'
                : '0 2px 0 rgba(255,255,255,0.8), 0 0 24px rgba(255,255,255,0.9)',
              WebkitTextStroke: theme.dark ? '2px rgba(0,0,0,0.35)' : undefined,
            }}
          >
            {w.text}
          </span>
        );
      })}
    </div>
  );
};
