// zensical.toml's search.highlight marks a search result's words on the page it opens (<mark data-md-highlight>)
// and leaves ?h=<query> in the URL, with no way to clear either: a reload keeps re-highlighting, and a copied
// link carries the query along. Esc, or this small "clear highlight" control, removes the marks and drops ?h=
// from the URL in the same action (history.replaceState), so a reload or a copied link afterwards is clean.
//
// The marks themselves arrive asynchronously (the built-in highlighter fetches the search index through a
// worker before it can match the query), so this watches for them with a MutationObserver instead of checking
// once on document$ - by then they usually haven't landed yet.
document$.subscribe(() => {
  if (!new URL(window.location.href).searchParams.has("h")) {
    return;
  }

  let button = null;
  let onKeydown = null;

  function clear() {
    for (const mark of document.querySelectorAll("mark[data-md-highlight]")) {
      mark.replaceWith(document.createTextNode(mark.textContent));
    }
    const url = new URL(window.location.href);
    url.searchParams.delete("h");
    history.replaceState(history.state, "", url);
    document.removeEventListener("keydown", onKeydown);
    button.remove();
    button = null;
  }

  function show() {
    if (button) {
      return;
    }
    onKeydown = (event) => {
      if (event.key === "Escape") {
        clear();
      }
    };
    document.addEventListener("keydown", onKeydown);

    button = document.createElement("button");
    button.type = "button";
    button.className = "eu-clear-highlight";
    button.textContent = "✕ clear highlight";
    button.addEventListener("click", clear);

    const content = document.querySelector(".md-content__inner");
    if (content) {
      content.prepend(button);
    }
  }

  if (document.querySelector("mark[data-md-highlight]")) {
    show();
    return;
  }

  const observer = new MutationObserver(() => {
    if (document.querySelector("mark[data-md-highlight]")) {
      observer.disconnect();
      show();
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });
});
