import {createContext, useContext} from 'react';
import {continueRender, delayRender} from 'remotion';
import interTight from './fonts/InterTight.woff2';
import jetbrainsMono from './fonts/JetBrainsMono.woff2';
import type {Format, ThemeName} from './types';

// Fonts ship with the project (variable woff2, SIL OFL) so renders never
// depend on the network and look identical on every machine.
export const fonts = {
  display: '"Inter Tight", "SF Pro Display", system-ui, sans-serif',
  mono: '"JetBrains Mono", "SF Mono", ui-monospace, monospace',
};

const loadFonts = () => {
  if (typeof document === 'undefined') return;
  const handle = delayRender('Loading fonts');
  const faces = [
    new FontFace('Inter Tight', `url(${interTight}) format('woff2')`, {weight: '100 900'}),
    new FontFace('JetBrains Mono', `url(${jetbrainsMono}) format('woff2')`, {weight: '100 800'}),
  ];
  Promise.all(faces.map((f) => f.load()))
    .then((loaded) => {
      loaded.forEach((f) => document.fonts.add(f));
      continueRender(handle);
    })
    .catch((err) => {
      console.error('Font loading failed', err);
      continueRender(handle);
    });
};
loadFonts();

export type Theme = {
  name: ThemeName;
  dark: boolean;
  bg: string;
  bg2: string;
  text: string;
  muted: string;
  faint: string;
  panel: string;
  panelBorder: string;
  accent: string;
  accentInk: string;
  accent2: string;
  good: string;
  bad: string;
  glowA: string;
  glowB: string;
  glowC: string;
  syntax: {
    keyword: string;
    string: string;
    number: string;
    comment: string;
    fn: string;
    type: string;
    punct: string;
  };
};

export const themes: Record<ThemeName, Theme> = {
  midnight: {
    name: 'midnight',
    dark: true,
    bg: '#05060B',
    bg2: '#0B0D1A',
    text: '#F5F6FB',
    muted: '#A3A9C2',
    faint: '#5A6080',
    panel: 'rgba(22, 25, 46, 0.72)',
    panelBorder: 'rgba(160, 170, 255, 0.14)',
    accent: '#C8FF4D',
    accentInk: '#0B1000',
    accent2: '#6E8BFF',
    good: '#4ADE80',
    bad: '#FF5C7A',
    glowA: '#3423A6',
    glowB: '#0E7C86',
    glowC: '#7A1FA2',
    syntax: {
      keyword: '#C792EA',
      string: '#C8FF4D',
      number: '#FFB86B',
      comment: '#5F6A8F',
      fn: '#7DD3FC',
      type: '#FFD866',
      punct: '#8E96B8',
    },
  },
  ember: {
    name: 'ember',
    dark: true,
    bg: '#0A0605',
    bg2: '#170C08',
    text: '#FFF4EC',
    muted: '#C9AFA2',
    faint: '#6E5248',
    panel: 'rgba(40, 20, 14, 0.72)',
    panelBorder: 'rgba(255, 170, 120, 0.16)',
    accent: '#FF8A3D',
    accentInk: '#1A0800',
    accent2: '#FFD166',
    good: '#7BE495',
    bad: '#FF4D5E',
    glowA: '#8A2A0B',
    glowB: '#6B1D3A',
    glowC: '#A85A00',
    syntax: {
      keyword: '#FF8A3D',
      string: '#FFD166',
      number: '#F78C6C',
      comment: '#7A5F55',
      fn: '#FFC6A5',
      type: '#E6A4FF',
      punct: '#B89A8E',
    },
  },
  paper: {
    name: 'paper',
    dark: false,
    bg: '#F4F1EA',
    bg2: '#EAE5DA',
    text: '#141414',
    muted: '#5D5A55',
    faint: '#AFA99E',
    panel: 'rgba(255, 255, 255, 0.82)',
    panelBorder: 'rgba(20, 20, 20, 0.10)',
    accent: '#2F5BFF',
    accentInk: '#FFFFFF',
    accent2: '#FF5A36',
    good: '#16A34A',
    bad: '#E11D48',
    glowA: '#C9D6FF',
    glowB: '#FFD9C9',
    glowC: '#D9F2E4',
    syntax: {
      keyword: '#7C3AED',
      string: '#047857',
      number: '#C2410C',
      comment: '#9C978C',
      fn: '#1D4ED8',
      type: '#B45309',
      punct: '#57534E',
    },
  },
};

// Layout boxes per output format. Vertical keeps content clear of the
// platform UI that covers the bottom and right edges of Reels, Shorts and
// TikTok.
export type Layout = {
  format: Format;
  width: number;
  height: number;
  content: {x: number; y: number; w: number; h: number};
  captionY: number;
  captionSize: number;
  titleSize: number;
  bodySize: number;
};

export const layoutFor = (format: Format, width: number, height: number): Layout => {
  if (format === 'vertical') {
    return {
      format,
      width,
      height,
      content: {x: 72, y: 250, w: width - 144, h: 980},
      captionY: 1320,
      captionSize: 78,
      titleSize: 112,
      bodySize: 50,
    };
  }
  if (format === 'square') {
    return {
      format,
      width,
      height,
      content: {x: 80, y: 110, w: width - 160, h: 650},
      captionY: 860,
      captionSize: 60,
      titleSize: 92,
      bodySize: 42,
    };
  }
  return {
    format,
    width,
    height,
    content: {x: 140, y: 110, w: width - 280, h: 700},
    captionY: 900,
    captionSize: 58,
    titleSize: 104,
    bodySize: 44,
  };
};

type Ctx = {theme: Theme; layout: Layout};

export const StyleContext = createContext<Ctx>({
  theme: themes.midnight,
  layout: layoutFor('vertical', 1080, 1920),
});

export const useStyle = () => useContext(StyleContext);
