import React, {useMemo} from 'react';
import {useCurrentFrame, useVideoConfig} from 'remotion';
import {fonts, useStyle} from '../theme';
import type {Word} from '../types';

type Page = {words: Word[]; start: number; end: number};

// Short caption pages that break on punctuation, pauses and length.
const paginate = (words: Word[], fps: number, maxChars: number): Page[] => {
  const pages: Page[] = [];
  let cur: Word[] = [];
  const flush = () => {
    if (cur.length) pages.push({words: cur, start: cur[0].start, end: cur[cur.length - 1].end});
    cur = [];
  };
  words.forEach((w, i) => {
    const prev = cur[cur.length - 1];
    const chars = cur.reduce((n, x) => n + x.text.length + 1, 0) + w.text.length;
    if (prev && (w.start - prev.end > fps * 0.4 || chars > maxChars)) flush();
    cur.push(w);
    if (/[.!?]$/.test(w.text) || !words[i + 1]) flush();
  });
  return pages;
};

// Burned-in captions in a quiet dark box, read like broadcast subtitles.
export const Captions: React.FC<{words: Word[]}> = ({words}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const {layout} = useStyle();
  const maxChars = layout.format === 'vertical' ? 30 : 46;
  const pages = useMemo(() => paginate(words, fps, maxChars), [words, fps, maxChars]);
  const page = pages.find((p, i) => {
    const next = pages[i + 1];
    const hold = next ? Math.min(next.start, p.end + fps * 0.6) : p.end + fps * 0.6;
    return frame >= p.start - 1 && frame < hold;
  });
  if (!page) return null;
  return (
    <div style={{position: 'absolute', left: 0, right: 0, top: layout.captionY, display: 'flex', justifyContent: 'center', padding: '0 120px'}}>
      <div
        style={{
          maxWidth: layout.format === 'vertical' ? 860 : 1180,
          textAlign: 'center',
          fontFamily: fonts.sans,
          fontSize: layout.captionSize,
          fontWeight: 600,
          lineHeight: 1.32,
          letterSpacing: '-0.01em',
          color: '#FFFFFF',
        }}
      >
        <span style={{background: 'rgba(28,28,26,0.86)', padding: '0.08em 0.3em', boxDecorationBreak: 'clone', WebkitBoxDecorationBreak: 'clone'}}>
          {page.words.map((w, i) => (
            <span key={i} style={{opacity: frame >= w.start ? 1 : 0.5}}>
              {i ? ' ' : ''}
              {w.text}
            </span>
          ))}
        </span>
      </div>
    </div>
  );
};
