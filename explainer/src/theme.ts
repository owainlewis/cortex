import {createContext, useContext} from 'react';
import {continueRender, delayRender} from 'remotion';
import geist from './fonts/Geist.woff2';
import geistMono from './fonts/GeistMono.woff2';
import serifItalic from './fonts/InstrumentSerifItalic.woff2';
import type {Format, ThemeName} from './types';

// Fonts ship with the project (SIL OFL) so renders never depend on the
// network and look identical on every machine.
export const fonts = {
  sans: '"Geist", "SF Pro Display", system-ui, sans-serif',
  serif: '"Instrument Serif", "New York", Georgia, serif',
  mono: '"Geist Mono", "SF Mono", ui-monospace, monospace',
};

const loadFonts = () => {
  if (typeof document === 'undefined') return;
  const handle = delayRender('Loading fonts');
  const faces = [
    new FontFace('Geist', `url(${geist}) format('woff2')`, {weight: '100 900'}),
    new FontFace('Geist Mono', `url(${geistMono}) format('woff2')`, {weight: '100 900'}),
    new FontFace('Instrument Serif', `url(${serifItalic}) format('woff2')`, {style: 'italic', weight: '400'}),
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

// Flat, restrained palettes: one ink, one paper, one accent. No gradients.
export type Theme = {
  name: ThemeName;
  dark: boolean;
  bg: string;
  surface: string;
  text: string;
  muted: string;
  faint: string;
  line: string;
  accent: string;
  accentInk: string;
  syntax: {keyword: string; string: string; number: string; comment: string; type: string};
};

export const themes: Record<ThemeName, Theme> = {
  ink: {
    name: 'ink',
    dark: true,
    bg: '#0F0F0E',
    surface: '#1A1A18',
    text: '#F2EFE8',
    muted: '#8F8C85',
    faint: '#4A4945',
    line: 'rgba(242, 239, 232, 0.16)',
    accent: '#FF5B1F',
    accentInk: '#0F0F0E',
    syntax: {keyword: '#FF7A47', string: '#D9C58F', number: '#D9C58F', comment: '#6E6C66', type: '#F2EFE8'},
  },
  paper: {
    name: 'paper',
    dark: false,
    bg: '#F1EDE4',
    surface: '#E6E1D5',
    text: '#141413',
    muted: '#6F6B63',
    faint: '#B9B4A8',
    line: 'rgba(20, 20, 19, 0.16)',
    accent: '#E5481B',
    accentInk: '#F1EDE4',
    syntax: {keyword: '#C23A12', string: '#5E6B2E', number: '#5E6B2E', comment: '#9A958A', type: '#141413'},
  },
  blueprint: {
    name: 'blueprint',
    dark: true,
    bg: '#0E1A33',
    surface: '#16244A',
    text: '#EAF0FF',
    muted: '#8D9BBE',
    faint: '#3A4A73',
    line: 'rgba(234, 240, 255, 0.18)',
    accent: '#FFD23F',
    accentInk: '#0E1A33',
    syntax: {keyword: '#FFD23F', string: '#9FD3C7', number: '#9FD3C7', comment: '#5E6E96', type: '#EAF0FF'},
  },
};

// Layout per format. Vertical keeps content clear of the platform UI that
// covers the bottom and right edges of Reels, Shorts and TikTok.
export type Layout = {
  format: Format;
  width: number;
  height: number;
  margin: number;
  content: {x: number; y: number; w: number; h: number};
  captionY: number;
  captionSize: number;
  display: number; // hook and statement type
  title: number; // scene headings
  body: number;
  small: number;
};

export const layoutFor = (format: Format, width: number, height: number): Layout => {
  if (format === 'vertical') {
    return {
      format, width, height, margin: 80,
      content: {x: 72, y: 250, w: width - 144, h: 980},
      captionY: 1350, captionSize: 68,
      display: 150, title: 92, body: 54, small: 30,
    };
  }
  if (format === 'square') {
    return {
      format, width, height, margin: 80,
      content: {x: 80, y: 130, w: width - 160, h: 620},
      captionY: 850, captionSize: 54,
      display: 100, title: 62, body: 40, small: 24,
    };
  }
  return {
    format, width, height, margin: 120,
    content: {x: 120, y: 130, w: width - 240, h: 660},
    captionY: 890, captionSize: 54,
    display: 112, title: 66, body: 40, small: 24,
  };
};

type Ctx = {theme: Theme; layout: Layout};

export const StyleContext = createContext<Ctx>({
  theme: themes.ink,
  layout: layoutFor('vertical', 1080, 1920),
});

export const useStyle = () => useContext(StyleContext);
