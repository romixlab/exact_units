// One variant of the logo on a page of its own size: `typst compile --input kind=mark --input theme=dark ...`.
// kind: lockup (mark over wordmark), mark, wordmark, icon (the mark on a dark tile), icon-small (the cube and its
// axes on the tile, for favicons), social (a 1280 × 640 card).
// theme: light (dark ink, for light pages) or dark (light ink, for dark pages); icon and social are always dark.
// size: the longer side in pt, which is pixels in a PNG at `--ppi 72`; empty or absent: the drawing's own size.
#import "logo.typ": *

#let kind = sys.inputs.at("kind", default: "lockup")
#let theme = sys.inputs.at("theme", default: "light")
#let size = sys.inputs.at("size", default: "")

#let (art, natural) = if kind == "lockup" { (lockup(theme: theme), lockup-size) } else if kind == "mark" {
  (mark(theme: theme), (width: mark-size, height: mark-size))
} else if kind == "wordmark" { (wordmark(theme: theme), wordmark-size) } else if kind in ("icon", "icon-small") {
  (icon(small: kind == "icon-small"), (width: icon-size, height: icon-size))
} else if kind == "social" {
  // The lockup on the dark tile colour, a fifth of the card's height clear above and below.
  let card = (width: 1280pt, height: 640pt)
  let f = card.height * 0.6 / lockup-size.height
  (block(..card, fill: tile-fill, align(center + horizon,
    scale(f * 100%, reflow: true, lockup(theme: "dark")))), card)
} else { panic("kind: lockup, mark, wordmark, icon, icon-small or social, not " + kind) }

#let f = if size == "" { 1 } else {
  float(size) * 1pt / calc.max(natural.width, natural.height)
}
#set page(width: natural.width * f, height: natural.height * f, margin: 0pt, fill: none)
#scale(f * 100%, origin: top + left, reflow: true, art)
