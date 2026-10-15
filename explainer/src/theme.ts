import {createContext, useContext} from 'react';
import {continueRender, delayRender} from 'remotion';
import geistMono from './fonts/GeistMono.woff2';
import serif from './fonts/InstrumentSerif.woff2';
import serifItalic from './fonts/InstrumentSerifItalic.woff2';
import manrope from './fonts/Manrope.woff2';
import type {Format, ThemeName} from './types';

// Fonts ship with the project (SIL OFL) so renders never depend on the
// network and look identical on every machine.
// Families go through CSS variables so a brand can swap them at render time.
export const fonts = {
  sans: 'var(--font-sans, "Manrope", "SF Pro Text", system-ui, sans-serif)',
  serif: 'var(--font-serif, "Instrument Serif", "New York", Georgia, serif)',
  mono: 'var(--font-mono, "Geist Mono", "SF Mono", ui-monospace, monospace)',
};

const loadFonts = () => {
  if (typeof document === 'undefined') return;
  const handle = delayRender('Loading fonts');
  const faces = [
    new FontFace('Manrope', `url(${manrope}) format('woff2')`, {weight: '200 800'}),
    new FontFace('Instrument Serif', `url(${serif}) format('woff2')`, {style: 'normal', weight: '400'}),
    new FontFace('Instrument Serif', `url(${serifItalic}) format('woff2')`, {style: 'italic', weight: '400'}),
    new FontFace('Geist Mono', `url(${geistMono}) format('woff2')`, {weight: '100 900'}),
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

// A calm studio look: warm canvas inside a soft frame, white stacked cards,
// one muted olive accent, black pills for emphasis.
export type Theme = {
  name: ThemeName;
  dark: boolean;
  frame: string;
  canvas: string;
  card: string;
  border: string;
  text: string;
  muted: string;
  faint: string;
  accent: string;
  accentSoft: string;
  accentInk: string;
  ink: string; // dark cards and pills
  inkText: string;
  inkMuted: string;
  shadow: string;
  captionBg: string;
  captionText: string;
  radius: number;
  // Style switches a brand can flip without touching components.
  surface: 'card' | 'outline';
  background: 'plain' | 'dots' | 'grid';
  labelColor: 'muted' | 'accent';
  labelFont: 'sans' | 'mono';
  arrows: 'curved' | 'straight';
  corners: boolean; // drafting ticks at card corners
  figures: boolean; // number scenes like figures in a drawing set
  divider: boolean; // rule, diamond, rule under headlines
  headlineWeight: number;
  syntax: {keyword: string; string: string; number: string; comment: string};
};

export const themes: Record<ThemeName, Theme> = {
  studio: {
    name: 'studio',
    dark: false,
    frame: '#E7E4DC',
    canvas: '#FBFAF7',
    card: '#FFFFFF',
    border: '#EAE7E0',
    text: '#1C1C1A',
    muted: '#8B887F',
    faint: '#CFCBC2',
    accent: '#566B39',
    accentSoft: '#EDF1E6',
    accentInk: '#FFFFFF',
    ink: '#171716',
    inkText: '#F4F3EF',
    inkMuted: '#8E8C86',
    shadow: '0 1px 2px rgba(30,28,20,0.04), 0 14px 36px rgba(30,28,20,0.07)',
    captionBg: 'rgba(28,28,26,0.86)',
    captionText: '#FFFFFF',
    radius: 22,
    surface: 'card',
    background: 'plain',
    labelColor: 'muted',
    labelFont: 'sans',
    arrows: 'curved',
    corners: false,
    figures: false,
    divider: false,
    headlineWeight: 400,
    syntax: {keyword: '#A5BE7C', string: '#D9C58F', number: '#D9C58F', comment: '#6E6C66'},
  },
  night: {
    name: 'night',
    dark: true,
    frame: '#0A0A09',
    canvas: '#141413',
    card: '#1D1D1B',
    border: '#2B2B28',
    text: '#F1EFE9',
    muted: '#8F8C84',
    faint: '#45443F',
    accent: '#A3BC79',
    accentSoft: 'rgba(163,188,121,0.14)',
    accentInk: '#141413',
    ink: '#F1EFE9',
    inkText: '#141413',
    inkMuted: '#6A6862',
    shadow: '0 1px 2px rgba(0,0,0,0.3), 0 14px 36px rgba(0,0,0,0.35)',
    captionBg: 'rgba(241,239,233,0.92)',
    captionText: '#141413',
    radius: 22,
    surface: 'card',
    background: 'plain',
    labelColor: 'muted',
    labelFont: 'sans',
    arrows: 'curved',
    corners: false,
    figures: false,
    divider: false,
    headlineWeight: 400,
    syntax: {keyword: '#566B39', string: '#7A5A1E', number: '#7A5A1E', comment: '#8E8C86'},
  },
};

export type Layout = {
  format: Format;
  width: number;
  height: number;
  inset: number; // frame around the canvas
  radius: number;
  // The canvas the graphics sit on, in frame coordinates.
  canvas: {x: number; y: number; w: number; h: number; radius: number};
  // Vertical footage splits the frame: graphics above `split`, speaker below.
  split: number | null;
  // True when the graphics area is wider than tall, so cards sit side by side.
  wide: boolean;
  content: {x: number; y: number; w: number; h: number};
  pip: {x: number; y: number; w: number; h: number};
  captionY: number;
  captionSize: number;
  display: number;
  title: number;
  body: number;
  small: number;
};

const framed = (width: number, height: number, inset: number, radius: number) => ({x: inset, y: inset, w: width - inset * 2, h: height - inset * 2, radius});

// `withSpeaker` reserves room for the speaker: picture-in-picture in landscape
// and square, a split screen in vertical.
export const layoutFor = (format: Format, width: number, height: number, withSpeaker = false): Layout => {
  if (format === 'vertical' && withSpeaker) {
    const split = Math.round(height * 0.47);
    return {
      format, width, height, inset: 0, radius: 0,
      canvas: {x: 0, y: 0, w: width, h: split, radius: 0},
      split,
      wide: true,
      content: {x: 64, y: 130, w: width - 128, h: split - 130 - 70},
      pip: {x: 0, y: split, w: width, h: height - split},
      captionY: split - 34, captionSize: 46,
      display: 104, title: 64, body: 38, small: 22,
    };
  }
  if (format === 'vertical') {
    return {
      format, width, height, inset: 18, radius: 36, canvas: framed(width, height, 18, 36), split: null, wide: false,
      content: {x: 72, y: 300, w: width - 144, h: 920},
      pip: {x: width - 18 - 40 - 230, y: 18 + 60, w: 230, h: 300},
      captionY: 1330, captionSize: 54,
      display: 140, title: 88, body: 48, small: 26,
    };
  }
  if (format === 'square') {
    return {
      format, width, height, inset: 16, radius: 30, canvas: framed(width, height, 16, 30), split: null, wide: false,
      content: {x: 80, y: 90, w: width - 160 - (withSpeaker ? 180 : 0), h: 700},
      pip: {x: width - 16 - 30 - 170, y: 300, w: 170, h: 230},
      captionY: 870, captionSize: 40,
      display: 96, title: 58, body: 32, small: 20,
    };
  }
  return {
    format, width, height, inset: 24, radius: 30, canvas: framed(width, height, 24, 30), split: null, wide: true,
    content: {x: 130, y: 90, w: width - 260 - (withSpeaker ? 250 : 0), h: 760},
    pip: {x: width - 24 - 44 - 240, y: 330, w: 240, h: 330},
    captionY: 890, captionSize: 54,
    display: 150, title: 96, body: 46, small: 26,
  };
};

type Ctx = {theme: Theme; layout: Layout};

export const StyleContext = createContext<Ctx>({
  theme: themes.studio,
  layout: layoutFor('landscape', 1920, 1080),
});

export const useStyle = () => useContext(StyleContext);

// Which scene is rendering, for figure numbers.
export const SceneIndex = createContext<{index: number; total: number}>({index: 0, total: 1});
