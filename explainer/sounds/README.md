# Sound effects

Put your own sound effects here, one folder per kind.
Any format ffmpeg reads works (wav, mp3, flac, aiff, m4a).
Kinds with no files fall back to the built-in synthesised sounds, so you can replace them one at a time.

| Folder | Plays when | What to look for |
| --- | --- | --- |
| `whoosh/` | a graphic comes in or leaves | short airy swish, 0.3 to 0.6 s, no long tail |
| `pop/` | a card, list item, step or bar appears | soft UI pop or tick, under 0.2 s |
| `impact/` | a stat lands, the opening hit | low boom or hit with a short tail |
| `riser/` | the build into a stat | 0.5 to 1 s riser that ends on its loudest point |
| `thud/` | the right side of a comparison lands | soft low thump |
| `key/` | code and terminal typing | single keyboard key clicks; add several for variety |
| `blip/` | code highlights, terminal output | short clean UI blip |
| `chime/` | the call to action | gentle bell or chime |

Files are peak-normalised, so you do not need to match levels.
Several files in one folder rotate in a fixed order, so every run of the same video sounds the same.
Trim silence from the start of each file: the sound plays the moment its event fires.

## Where to get good ones

Check the licence of every file you download before you publish with it.

- [Freesound](https://freesound.org): huge library; filter by the CC0 licence to avoid attribution requirements.
- [Pixabay sound effects](https://pixabay.com/sound-effects/): free under the Pixabay Content License, which allows commercial use.
- [Mixkit](https://mixkit.co/free-sound-effects/): small curated set of free UI and transition sounds.
- [Kenney](https://kenney.nl/assets?q=audio): CC0 UI and interface sound packs.
- [Sonniss GDC bundles](https://sonniss.com/gameaudiogdc): free yearly royalty-free bundles of professional effects.
- Paid, highest quality and cleared for social: [Epidemic Sound](https://www.epidemicsound.com), [Artlist](https://artlist.io), [Soundsnap](https://www.soundsnap.com).

Audio files in this folder are git-ignored; only this README is tracked.
