# Vendored Phosphor Icons

Source: `phosphor-icons/core` at commit `2b75f3ad12b420c9504ef05df8d2564a28f8500e` (https://github.com/phosphor-icons/core).

Included unmodified **1,512 regular-weight SVGs** in `regular/`. `icons.txt` lists every usable icon name; `src/svg_icons/catalog*.rs` maps names to embedded assets. The accompanying upstream MIT notice is in `LICENSE`. Icons are embedded as vector paths at build time; no icon font or remote request is required at render time.

When an icon is actually rendered, LAML writes the notice into nonvisible SVG `<metadata>`, PNG `iTXt` (`License`), and PDF `Info.Subject`. Exports without icons do not receive a Phosphor notice. This is an engineering attribution policy; publication may require independent legal review.
