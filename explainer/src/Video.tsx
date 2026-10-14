import React from 'react';
import {AbsoluteFill, Audio, Sequence, staticFile} from 'remotion';
import {Background} from './components/Background';
import {Captions} from './components/Captions';
import {EXIT_FRAMES, ProgressBar, SceneShell} from './components/Chrome';
import {Bars, Compare, List} from './scenes/CardScenes';
import {Code, Terminal} from './scenes/CodeScenes';
import {Flow, Tree} from './scenes/DiagramScenes';
import {Hook, Outro, Stat, Statement} from './scenes/TextScenes';
import {fonts, layoutFor, StyleContext, themes} from './theme';
import type {SceneProps, Timeline} from './types';

const SCENES: Record<string, React.FC<SceneProps & {handle?: string | null}>> = {
  hook: Hook,
  statement: Statement,
  stat: Stat,
  list: List,
  compare: Compare,
  bars: Bars,
  code: Code,
  terminal: Terminal,
  flow: Flow,
  tree: Tree,
  outro: Outro,
};

export const Explainer: React.FC<Timeline> = (timeline) => {
  const theme = themes[timeline.theme] ?? themes.midnight;
  const layout = layoutFor(timeline.format, timeline.width, timeline.height);
  return (
    <StyleContext.Provider value={{theme, layout}}>
      <AbsoluteFill style={{background: theme.bg, fontFamily: fonts.display, color: theme.text}}>
        <Background scenes={timeline.scenes} />
        {timeline.scenes.map((scene, i) => {
          const Scene = SCENES[scene.type];
          const last = i === timeline.scenes.length - 1;
          return (
            <Sequence
              key={i}
              from={scene.from}
              durationInFrames={scene.duration + (last ? 0 : EXIT_FRAMES)}
              name={`${i + 1}. ${scene.type}`}
            >
              <SceneShell scene={scene} last={last}>
                <Scene scene={scene} index={i} handle={timeline.handle} />
              </SceneShell>
            </Sequence>
          );
        })}
        <Captions scenes={timeline.scenes} />
        <ProgressBar scenes={timeline.scenes} handle={timeline.handle} />
        {timeline.audio ? <Audio src={staticFile(timeline.audio)} /> : null}
      </AbsoluteFill>
    </StyleContext.Provider>
  );
};
