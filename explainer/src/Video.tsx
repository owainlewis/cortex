import React from 'react';
import {AbsoluteFill, Audio, Sequence, staticFile} from 'remotion';
import {Captions} from './components/Captions';
import {Frame, SceneShell, Speaker} from './components/Chrome';
import {Code, Terminal} from './scenes/CodeScenes';
import {ArrayScene, Flow, Tree} from './scenes/DiagramScenes';
import {Bars, Compare, List} from './scenes/ListScenes';
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
  array: ArrayScene,
  outro: Outro,
};

export const Explainer: React.FC<Timeline> = (timeline) => {
  const theme = themes[timeline.theme] ?? themes.studio;
  const layout = layoutFor(timeline.format, timeline.width, timeline.height, Boolean(timeline.video));
  const captionWords = timeline.video
    ? timeline.words
    : timeline.scenes.flatMap((s) => (s.captions ? s.words.map((w) => ({...w, start: w.start + s.from, end: w.end + s.from})) : []));
  return (
    <StyleContext.Provider value={{theme, layout}}>
      <AbsoluteFill style={{fontFamily: fonts.sans, color: theme.text}}>
        <Frame>
          {timeline.scenes.map((scene, i) => {
            const Scene = SCENES[scene.type];
            return (
              <Sequence key={i} from={scene.from} durationInFrames={scene.duration} name={`${i + 1}. ${scene.type}`}>
                <SceneShell scene={scene} last={i === timeline.scenes.length - 1 && !timeline.video}>
                  <Scene scene={scene} index={i} handle={timeline.handle} />
                </SceneShell>
              </Sequence>
            );
          })}
        </Frame>
        {timeline.video ? <Speaker src={timeline.video} scenes={timeline.scenes} focus={timeline.focus} /> : null}
        <Captions words={captionWords} />
        {timeline.audio ? <Audio src={staticFile(timeline.audio)} /> : null}
      </AbsoluteFill>
    </StyleContext.Provider>
  );
};
