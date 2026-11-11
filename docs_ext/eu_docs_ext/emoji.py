"""The emoji generator of the book (zensical.toml's `pymdownx.emoji`): Zensical's own `to_svg`, with the
shortcode the page wrote kept on the element as `data-emoji`. Zensical draws an icon (`:material-...:`) as a
`<span class="twemoji">` around an inline SVG and drops its name; with the name kept, eu_docs_ext.check_emoji can
tell from the built site which shortcode each emoji and icon came from, and fail on one the book never meant
(`hh:mm:ss` drawn as `:mm:`, the flag of Myanmar).
"""

from zensical.extensions.emoji import to_svg as _zensical_to_svg


def to_svg(index, shortname, alias, uc, alt, title, category, options, md):  # noqa: PLR0913, PLR0917
    el = _zensical_to_svg(index, shortname, alias, uc, alt, title, category, options, md)
    # as the page wrote it: `:mm:` is an alias of `:flag_mm:`
    el.set("data-emoji", alias or shortname)
    return el
