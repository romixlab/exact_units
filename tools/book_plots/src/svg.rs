//! A small SVG plot: axes with ticks, lines and points, styled by the book's classes (`.eu-svg`, `.line`, `.accent`,
//! ... in docs/stylesheets/book.css), so a plot inlined into a page follows the light or dark scheme.

use core::fmt::Write;

/// One panel: data ranges mapped onto a box of the drawing.
pub struct Panel {
    /// left, top, width, height of the plotting area in SVG units
    pub area: (f64, f64, f64, f64),
    pub x: (f64, f64),
    pub y: (f64, f64),
}

impl Panel {
    pub fn px(&self, x: f64) -> f64 {
        let (l, _, w, _) = self.area;
        l + (x - self.x.0) / (self.x.1 - self.x.0) * w
    }

    pub fn py(&self, y: f64) -> f64 {
        let (_, t, _, h) = self.area;
        t + h - (y - self.y.0) / (self.y.1 - self.y.0) * h
    }
}

/// An SVG document being written.
pub struct Svg {
    out: String,
}

impl Svg {
    pub fn new(width: f64, height: f64, label: &str) -> Self {
        let mut out = String::new();
        let _ = write!(
            out,
            r#"<svg class="eu-svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" role="img" aria-label="{label}" xmlns="http://www.w3.org/2000/svg">"#
        );
        Svg { out }
    }

    pub fn finish(mut self) -> String {
        self.out.push_str("</svg>\n");
        self.out
    }

    pub fn text(&mut self, x: f64, y: f64, anchor: &str, class: &str, s: &str) {
        let _ = write!(
            self.out,
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" class="{class}">{s}</text>"#
        );
    }

    pub fn line(&mut self, a: (f64, f64), b: (f64, f64), class: &str) {
        let _ = write!(
            self.out,
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="{class}"/>"#,
            a.0, a.1, b.0, b.1
        );
    }

    pub fn circle(&mut self, c: (f64, f64), r: f64, class: &str) {
        let _ = write!(
            self.out,
            r#"<circle cx="{:.1}" cy="{:.1}" r="{r}" class="{class}"/>"#,
            c.0, c.1
        );
    }

    /// A polyline through data points of a panel.
    pub fn path(&mut self, p: &Panel, points: &[(f64, f64)], class: &str) {
        let mut d = String::new();
        for (i, &(x, y)) in points.iter().enumerate() {
            let _ = write!(
                d,
                "{}{:.1},{:.1} ",
                if i == 0 { "M" } else { "L" },
                p.px(x),
                p.py(y)
            );
        }
        let _ = write!(self.out, r#"<path d="{}" class="{class}"/>"#, d.trim_end());
    }

    /// The axes of a panel: a frame on the left and bottom, ticks with their labels, and the axis titles.
    pub fn axes(
        &mut self,
        p: &Panel,
        xticks: &[(f64, &str)],
        yticks: &[(f64, &str)],
        xtitle: &str,
        ytitle: &str,
    ) {
        let (l, t, w, h) = p.area;
        self.line((l, t + h), (l + w, t + h), "thin");
        self.line((l, t), (l, t + h), "thin");
        for &(x, s) in xticks {
            let px = p.px(x);
            self.line((px, t + h), (px, t + h + 4.0), "thin");
            self.text(px, t + h + 17.0, "middle", "soft", s);
        }
        for &(y, s) in yticks {
            let py = p.py(y);
            self.line((l - 4.0, py), (l, py), "thin");
            self.line((l, py), (l + w, py), "thin dash grid");
            self.text(l - 7.0, py + 4.0, "end", "soft", s);
        }
        if !xtitle.is_empty() {
            self.text(l + w / 2.0, t + h + 36.0, "middle", "", xtitle);
        }
        if !ytitle.is_empty() {
            self.text(l, t - 10.0, "start", "", ytitle);
        }
    }
}
