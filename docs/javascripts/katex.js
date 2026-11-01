// Math on every page, also after instant navigation: pymdownx.arithmatex (generic) wraps $...$ in \( \) and $$...$$
// in \[ \], KaTeX draws them.
document$.subscribe(({ body }) => {
  renderMathInElement(body, {
    delimiters: [
      { left: "\\(", right: "\\)", display: false },
      { left: "\\[", right: "\\]", display: true },
    ],
    throwOnError: false,
  });
});
