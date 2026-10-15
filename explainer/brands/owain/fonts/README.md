# Fonts

The brand is set in Signifier (display) and Helvetica Neue (text).
Both are licensed fonts, so they are not in this repository.

Until you add them, renders use free stand-ins with similar proportions:
`Newsreader-standin.woff2` for Signifier and `Inter-standin.woff2` for Helvetica.

To use the real fonts, edit `fonts` in `../brand.json`:

```json
"fonts": {
  "serif": "fonts/Signifier-Light.woff2",
  "serifItalic": "fonts/Signifier-LightItalic.woff2",
  "sans": "local:Helvetica Neue"
}
```

- Put your licensed Signifier files (woff2, otf or ttf) in this folder; they are git-ignored.
- `local:Helvetica Neue` uses the copy installed with macOS, so no file is needed when you render on a Mac.
- Check that your Signifier licence covers use in video; Klim sells desktop and web licences separately.
