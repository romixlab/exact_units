// Math on every page, also after instant navigation: pymdownx.arithmatex (generic) wraps $...$ in \( \) and $$...$$
// in \[ \], KaTeX draws them.
//
// \unit{...}: a unit symbol inside math (SI Brochure 9th §5.1 / ISO 80000-1: unit symbols upright, quantity and
// constant symbols italic), \mathrm so it doesn't come out slanted like a variable. Its argument is checked
// against q!'s table at build time the same way [[..]] is (eu_docs_ext.check_unit_macros, `just docs-build`).
document$.subscribe(({ body }) => {
  renderMathInElement(body, {
    delimiters: [
      { left: "\\(", right: "\\)", display: false },
      { left: "\\[", right: "\\]", display: true },
    ],
    macros: { "\\unit": "\\mathrm{#1}" },
    throwOnError: false,
  });
});
