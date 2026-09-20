# Grünes Gewölbe identity

The default mark is **Coral vault** (`logo.svg`): a branching coral form beneath a warm gold arch, on a phthalo-green background. The branching form evokes coral ornamentation.

## Options

- `logo.svg` — Coral vault, used by the project and extension.
- `option-cabinet.svg` — Cabinet seal, coral inside a faceted frame.
- `option-branching-g.svg` — Branching G, a letterform and coral silhouette.
- `options.svg` — self-contained comparison sheet, including small-size previews.

Palette: phthalo-green interpretation `#123D32`, coral `#F17B67`, warm gold `#DEC99D`, ivory `#F4F0E7`. These are screen colors, not a pigment color-matching specification.

All marks are editable, self-contained SVGs with no fonts or external resources. The comparison sheet uses local system fonts for captions. Browser toolbar/installation icons are committed PNG exports, so loading the unpacked extension requires no build step.

Regenerate icons after editing `logo.svg`:

```sh
node scripts/build-icons.mjs
```
