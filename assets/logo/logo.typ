// The exact_units logo: the mark (a dial around a cube with its three axes) and the wordmark.
// One grid drives the mark: a 12-position dial (every 30°) and a regular isometric cube on the same centre.
// One weight and one gap drive every stroke and every cut, so all clearances match.
// render.typ picks a variant and a size; `just logo` exports them all.

#let orange = rgb("#ff7100")
#let themes = (
  light: (ink: rgb("#373c42"), edge: rgb("#69717a"), top: rgb("#eef0f2"), left: (rgb("#e4e7eb"), rgb("#fafafa"))),
  dark: (ink: rgb("#dddddd"), edge: rgb("#a6a8aa"), top: rgb("#202326"), left: (rgb("#25292c"), rgb("#171a1d"))),
)
// The icon's tile and the social card are dark, whatever the page around them.
#let tile-fill = rgb("#0f0f10")

#let w = 22pt            // main weight: ring, bars, shafts (the lettering's stem is ~24pt)
#let gap = 12pt          // one clearance for every cut and every separation
#let ring-r = 254pt      // ring centreline
#let bar-len = 70pt      // cardinal bars, centred on the ring
#let tick-w = 14pt
#let tick-len = 32pt
#let rho = 142pt         // cube: circumradius of its hexagon
#let edge = 16pt         // cube outline
#let head-w = 64pt
#let head-len = 54pt

// The mark's box: the bars reach furthest out.
#let mark-r = ring-r + bar-len / 2
#let mark-size = 2 * mark-r
#let c = (mark-r, mark-r)

// Screen angles, y down: 270deg is up. The three arrows point away from the cube's alternate corners.
#let bars = (0deg, 90deg, 180deg)
#let arrows = (30deg, 150deg, 270deg)
#let ticks = (60deg, 120deg, 240deg, 300deg)

#let at(r, a) = (c.at(0) + r * calc.cos(a), c.at(1) + r * calc.sin(a))
// A point in an element's own frame: d along its axis from the centre, s across it (+s towards larger angles).
#let local(a, d, s) = (d * calc.cos(a) - s * calc.sin(a), d * calc.sin(a) + s * calc.cos(a))

// The arrowhead straddles the ring: its tip clears the outer edge, its base sits inside the inner edge.
#let tip = ring-r + w / 2 + 22pt
#let base = tip - head-len
// The tail is a chevron one gap outside the outline's corner: the outline's outer edges sit edge/2 off the
// hexagon's sides, which meet at 120°, so the tail's apex is (edge/2 + gap) / sin 60° beyond the corner.
#let apex = rho + (edge / 2 + gap) / calc.sin(60deg)
#let shoulder = apex - w / 2 / calc.tan(60deg)

// Without the dial (the small icon) the arrows are shorter, so the cube can be drawn larger.
#let tip-bare = 262pt
#let arrow(a, tip: tip) = {
  let p(d, s) = { let (x, y) = local(a, d, s); (c.at(0) + x, c.at(1) + y) }
  let base = tip - head-len
  (p(apex, 0pt), p(shoulder, -w / 2), p(base, -w / 2), p(base, -head-w / 2), p(tip, 0pt),
    p(base, head-w / 2), p(base, w / 2), p(shoulder, w / 2))
}

// Ring cuts. Each cut is a line one gap off the element beside it: parallel to a bar's side, or parallel to the
// near side of an arrowhead, so the ring's ends follow the arrows. Lines are (point, direction) relative to the
// centre in plain pt numbers; side is -1 for the cut that ends the segment before the element, +1 after it.
#let cut-line(kind, a, side) = {
  let f(v) = v.map(x => x / 1pt)
  if kind == "bar" {
    (f(local(a, 0pt, side * (w / 2 + gap))), (calc.cos(a), calc.sin(a)))
  } else {
    // The head's side runs from (base, ±head-w/2) to (tip, 0); its outward normal is (head-w/2, tip - base).
    let len = calc.sqrt(calc.pow((head-w / 2) / 1pt, 2) + calc.pow((tip - base) / 1pt, 2))
    let n-along = (head-w / 2) / 1pt / len
    let n-across = (tip - base) / 1pt / len
    let p = local(a, base + gap * n-along, side * (head-w / 2 + gap * n-across))
    let dir = local(a, tip - base, -side * head-w / 2)
    (f(p), f(dir).map(x => x / len))
  }
}
// The angle (in degrees, unwrapped near `a`) where a cut line crosses the circle of radius r.
#let cut-angle(line, r, a) = {
  let ((qx, qy), (ux, uy)) = line
  let qu = qx * ux + qy * uy
  let disc = calc.sqrt(qu * qu - (qx * qx + qy * qy) + r * r)
  let near(l) = {
    let deg = calc.atan2(qx + l * ux, qy + l * uy).deg()
    a.deg() + calc.rem-euclid(deg - a.deg() + 180, 360) - 180
  }
  let (x, y) = (near(-qu + disc), near(-qu - disc))
  if calc.abs(x - a.deg()) < calc.abs(y - a.deg()) { x } else { y }
}
#let ring-segments() = {
  let els = (bars.map(a => ("bar", a)) + arrows.map(a => ("arrow", a))).sorted(key: e => e.at(1))
  let segs = ()
  for i in range(els.len()) {
    let (k1, a1) = els.at(i)
    let (k2, a2) = els.at(calc.rem(i + 1, els.len()))
    if a2 <= a1 { a2 += 360deg }
    let pts = ()
    for (r, rev) in ((ring-r + w / 2, false), (ring-r - w / 2, true)) {
      let s = cut-angle(cut-line(k1, a1, 1), r / 1pt, a1) * 1deg
      let e = cut-angle(cut-line(k2, a2, -1), r / 1pt, a2) * 1deg
      for j in range(121) {
        let k = if rev { 120 - j } else { j }
        pts.push(at(r, s + (e - s) * k / 120))
      }
    }
    segs.push(pts)
  }
  segs
}

// The mark alone, mark-size square, transparent around it. Without the dial: the cube and its axes only, for
// sizes where the ring and the ticks would blur.
#let mark(theme: "light", dial: true) = block(width: mark-size, height: mark-size, {
  let t = themes.at(theme)
  if dial {
    for pts in ring-segments() { place(top + left, polygon(..pts, fill: t.ink, stroke: none)) }
    // Cardinal bars, centred on the ring.
    for a in bars {
      place(top + left, line(start: at(ring-r - bar-len / 2, a), end: at(ring-r + bar-len / 2, a),
        stroke: (paint: orange, thickness: w, cap: "butt")))
    }
    // Ticks, one gap inside the ring.
    for a in ticks {
      let r1 = ring-r - w / 2 - gap
      place(top + left, line(start: at(r1 - tick-len, a), end: at(r1, a),
        stroke: (paint: t.ink, thickness: tick-w, cap: "butt")))
    }
  }
  // Cube: a regular hexagon split into three faces at the centre.
  let v = range(6).map(i => at(rho, -90deg + i * 60deg))  // top, upper right, lower right, bottom, lower left, upper left
  let o = c
  place(top + left, polygon(fill: t.top, stroke: none, v.at(0), v.at(1), o, v.at(5)))
  place(top + left, polygon(fill: gradient.linear(..t.left, angle: 30deg), stroke: none,
    v.at(5), o, v.at(3), v.at(4)))
  place(top + left, polygon(fill: gradient.linear(rgb("#ff8a08"), rgb("#ff6500"), angle: 125deg), stroke: none,
    o, v.at(1), v.at(2), v.at(3)))
  // The outline is one closed path and the Y is one path, so every corner is a clean miter.
  let st = (paint: t.edge, thickness: edge, join: "miter", cap: "butt")
  place(top + left, curve(fill: none, stroke: st, curve.move(v.at(0)),
    ..v.slice(1).map(p => curve.line(p)), curve.close()))
  place(top + left, curve(fill: none, stroke: st, curve.move(v.at(5)), curve.line(o), curve.line(v.at(1))))
  place(top + left, line(start: o, end: v.at(3), stroke: st))
  // Three axes.
  for a in arrows {
    place(top + left, polygon(..arrow(a, tip: if dial { tip } else { tip-bare }), fill: t.ink, stroke: none))
  }
})

// The wordmark: Audiowide, each word widened to the lettering's proportions, the underscore an orange bar.
#let wordmark-size = (width: 1268pt, height: 140pt)
#let wordmark(theme: "light") = block(..wordmark-size, {
  let ink = themes.at(theme).ink
  for (label, x, wd) in (("exact", 0pt, 613pt), ("units", 745pt, 523pt)) {
    let word = text(font: "Audiowide", size: 190pt, fill: ink, label)
    // The glyphs rise 9pt above the cap height ('t'); this puts their top on the block's top.
    context {
      let size = measure(word)
      place(top + left, dx: x, dy: 9pt, scale(x: wd / size.width * 100%, y: 100%, origin: top + left, word))
    }
  }
  place(top + left, dx: 632pt, dy: 118pt, rect(width: 97pt, height: 22pt, fill: orange, stroke: none))
})

// The mark above the wordmark.
#let lockup-gap = 56pt
#let lockup-size = (width: wordmark-size.width, height: mark-size + lockup-gap + wordmark-size.height)
#let lockup(theme: "light") = block(..lockup-size, {
  place(top + center, mark(theme: theme))
  place(bottom + left, wordmark(theme: theme))
})

// The app icon: the dark mark on a dark rounded tile, for avatars and rustdoc, on any background. The small one
// drops the dial and fills the tile with the cube, for favicons and sizes up to 48 px.
#let icon-size = 680pt
#let icon(small: false) = block(width: icon-size, height: icon-size, radius: 22%, fill: tile-fill, clip: true,
  align(center + horizon, if small {
    scale((icon-size / 2 - 42pt) / tip-bare * 100%, reflow: true, mark(theme: "dark", dial: false))
  } else { mark(theme: "dark") }))
