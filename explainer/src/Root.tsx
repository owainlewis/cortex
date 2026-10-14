import React from 'react';
import {CalculateMetadataFunction, Composition} from 'remotion';
import sample from './sample-timeline.json';
import type {Timeline} from './types';
import {Explainer} from './Video';

const calculateMetadata: CalculateMetadataFunction<Timeline> = ({props}) => ({
  durationInFrames: props.durationInFrames,
  fps: props.fps,
  width: props.width,
  height: props.height,
});

export const Root: React.FC = () => (
  <Composition
    id="Explainer"
    component={Explainer}
    defaultProps={sample as unknown as Timeline}
    calculateMetadata={calculateMetadata}
    durationInFrames={300}
    fps={30}
    width={1080}
    height={1920}
  />
);
