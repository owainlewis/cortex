import React, {useEffect, useState} from 'react';
import {continueRender, delayRender, Img, staticFile} from 'remotion';
import {useStyle} from '../theme';
import type {Timeline} from '../types';

type Fonts = NonNullable<NonNullable<Timeline['brand']>['fonts']>;

const FAMILY = {sans: 'Brand Sans', serif: 'Brand Serif', serifItalic: 'Brand Serif', mono: 'Brand Mono'} as const;
const FALLBACK = {
  sans: '"Manrope", system-ui, sans-serif',
  serif: '"Instrument Serif", Georgia, serif',
  mono: '"Geist Mono", ui-monospace, monospace',
};

// A font value is either a file in the public dir or `local:Family Name` for a
// font installed on the machine doing the render (e.g. Helvetica Neue on macOS).
const isLocal = (v?: string) => Boolean(v && v.startsWith('local:'));
const localName = (v: string) => v.slice('local:'.length).trim();

// Loads brand font files and returns CSS variables that point the theme's font
// stacks at them. Rendering waits until the fonts are ready.
export const useBrandFonts = (files: Fonts | undefined): React.CSSProperties => {
  const toLoad = (Object.keys(files ?? {}) as (keyof Fonts)[]).filter((k) => files![k] && !isLocal(files![k]));
  const [handle] = useState(() => (toLoad.length ? delayRender('Loading brand fonts') : null));
  useEffect(() => {
    if (handle === null || !files) return;
    const faces = toLoad.map(
      (key) =>
        new FontFace(FAMILY[key], `url(${staticFile(files[key]!)})`, {
          weight: '100 900',
          style: key === 'serifItalic' ? 'italic' : 'normal',
        }),
    );
    Promise.all(faces.map((f) => f.load()))
      .then((loaded) => loaded.forEach((f) => document.fonts.add(f)))
      .catch((err) => console.error('Brand font failed to load', err))
      .finally(() => continueRender(handle));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [files, handle]);
  const vars: Record<string, string> = {};
  for (const key of ['sans', 'serif', 'mono'] as const) {
    const v = files?.[key];
    if (!v) continue;
    const family = isLocal(v) ? `"${localName(v)}"` : `"${FAMILY[key]}"`;
    vars[`--font-${key}`] = `${family}, ${FALLBACK[key]}`;
  }
  return vars as React.CSSProperties;
};

// Brand mark in the canvas's top-left corner.
export const Logo: React.FC<{src: string}> = ({src}) => {
  const {layout} = useStyle();
  const h = layout.format === 'vertical' ? 54 : 46;
  return (
    <Img
      src={staticFile(src)}
      style={{position: 'absolute', left: layout.canvas.x + 34, top: layout.canvas.y + 30, height: h, width: 'auto', objectFit: 'contain', opacity: 0.92}}
    />
  );
};
