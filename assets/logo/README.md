# SolideoWorship logo

The mark is the **Lyric Cross**: centered lines of lyrics whose silhouette forms a cross. The words on the screen, held by the cross.

## Files

| File | Use |
| --- | --- |
| `lockup-on-light.svg` / `lockup-on-dark.svg` | Mark and name, for light or dark backgrounds |
| `lockup-stacked-on-light.svg` / `lockup-stacked-on-dark.svg` | Stacked version for square spaces |
| `lockup-black.svg` / `lockup-white.svg` | One-colour print, engraving, stamps |
| `symbol-on-light.svg` / `symbol-on-dark.svg` | Mark alone, transparent background |
| `symbol-black.svg` / `symbol-white.svg` | Mark alone, one colour |
| `app-icon.svg` / `app-icon-light.svg` | App icon tile, 48 px and up |
| `app-icon-32.svg` | App icon cut for 32 px |
| `app-icon-24.svg` | App icon cut for 24 px |
| `app-icon-16.svg` | App icon cut for 16 px |
| `installer-wizard.svg` | Side image of the Windows installer |
| `social-preview.svg` | GitHub social preview, 1280×640 |
| `png/`, `solideoworship.ico` | Rendered by `render.sh`; do not edit by hand |

## Colours

| Name | Hex | Use |
| --- | --- | --- |
| Ink | `#1C1D21` | Mark and text on light backgrounds, icon tile |
| Paper | `#F2F2F3` | Mark and text on dark backgrounds |
| Mist | `#E9EAEC` | Light icon tile |

Inside the app the mark takes the theme's text colour.

## Rules

- Keep clear space of at least one crossbar height (two lines and their gap) around the mark.
- Smallest sizes: 16 px for the icon cuts, 24 px tall for the full mark, 120 px wide for the horizontal lockup.
- Never rotate, mirror, stretch, outline, or add shadows or gradients. Rotated 180°, a Latin cross reads as an inverted cross.
- Never redraw the name in another font. It is outlined from [Plus Jakarta Sans](https://github.com/tokotype/PlusJakartaSans) (SIL Open Font License), "Solideo" at weight 760 and "Worship" at 420.

## Regenerating

Edit the SVGs, then run `./render.sh` (needs `rsvg-convert` and `python3`) and commit the results. The row tables in `crates/app/src/logo.rs` mirror `symbol-*.svg` and `app-icon-32.svg`, and its window icon mirrors `app-icon-16.svg`.
