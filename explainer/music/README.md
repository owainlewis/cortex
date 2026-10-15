# Music

Put music tracks here.
A video picks one track from `music/<mood>/` (moods are `focus`, `uplift` and `dark`), or from `music/` itself when that mood has no folder.
The choice is fixed per video name, so re-rendering keeps the same track and different videos spread across your library.
Pin a track for one video with `"music": {"track": "my-track.mp3"}` in its spec or plan.

Tracks are looped with a crossfade when the video is longer, faded out at the end, levelled, and ducked under the voice automatically.
Set how loud music sits under speech with `musicLevel` in `brand/brand.json` (decibels, default -20 for footage).
With no tracks here, the pipeline synthesises an original bed.

Instrumental tracks without strong melodies work best under speech.

## Where to get good ones

Check the licence covers the platforms you post to.

- [Epidemic Sound](https://www.epidemicsound.com) and [Artlist](https://artlist.io): subscription libraries cleared for YouTube, TikTok and Instagram, with stems.
- [Pixabay music](https://pixabay.com/music/): free under the Pixabay Content License.
- [YouTube Audio Library](https://studio.youtube.com): free tracks, cleared for YouTube; check terms for other platforms.

Audio files in this folder are git-ignored; only this README is tracked.
