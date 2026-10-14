// Render selected frames of a timeline as PNGs: a fast design-review loop.
//   node scripts/stills.mjs <timeline.json> <public-dir> <out-dir> [at=0.5,0.95]
// Writes two stills per scene, at the given fractions of each scene.
import {bundle} from '@remotion/bundler';
import {renderStill, selectComposition} from '@remotion/renderer';
import fs from 'node:fs';
import path from 'node:path';

const [timelinePath, publicDir, outDir, at = '0.5,0.95'] = process.argv.slice(2);
const props = JSON.parse(fs.readFileSync(timelinePath, 'utf8'));
const fractions = at.split(',').map(Number);
fs.mkdirSync(outDir, {recursive: true});

const serveUrl = await bundle({entryPoint: path.resolve('src/index.ts'), publicDir: path.resolve(publicDir)});
const browserExecutable = process.env.REMOTION_BROWSER_EXECUTABLE || null;
const composition = await selectComposition({serveUrl, id: 'Explainer', inputProps: props, browserExecutable});

for (const [i, scene] of props.scenes.entries()) {
  for (const f of fractions) {
    const frame = Math.min(scene.from + Math.round(scene.duration * f), props.durationInFrames - 1);
    const output = path.join(outDir, `${String(i + 1).padStart(2, '0')}-${scene.type}-${Math.round(f * 100)}.png`);
    await renderStill({serveUrl, composition, frame, output, inputProps: props, browserExecutable});
    console.log(output);
  }
}
