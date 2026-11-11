"""q!'s symbol table, read from docs/start/symbols.md's generated tables (`just q-symbols` writes them, from
macros/src/pure/table.rs), and resolved the way `q!` reads a unit: a bare symbol, one with a prefix it takes
(`keV`), or a compound of symbols joined by `/` and `·` with an exponent (`mSv/h`, `m/s²`).

Used by both `unit_markup` (the `[[symbol]]` prose markup) and `check_unit_macros` (the `\\unit{}` KaTeX macro), so
a page and a formula are checked against the same table the same way. An unknown or ambiguous symbol is a build
failure (`UnitSymbolError`), never a guess: the crate's own "exact, or refused" (AGENTS.md).
"""

from __future__ import annotations

import functools
import re
from dataclasses import dataclass
from pathlib import Path

from eu_docs_ext.unit_info import LEVEL_INFO, ORIGIN_INFO, PREFIX_INFO, UNIT_INFO

SYMBOLS_PAGE = Path(__file__).resolve().parents[2] / "docs" / "start" / "symbols.md"

_TABLE = re.compile(r"<!-- generated: (\w+) -->\n(.*?)\n<!-- end: \1 -->", re.DOTALL)
_BACKTICK = re.compile(r"`([^`]*)`")
_SUPERSCRIPT = "⁰¹²³⁴⁵⁶⁷⁸⁹⁻"


class UnitSymbolError(ValueError):
    """An unknown or ambiguous symbol inside `[[..]]` or `\\unit{}`: fails the docs build."""


@dataclass(frozen=True)
class Factor:
    """One resolved unit, origin or level: `mSv` in `[[mSv/h]]`, with the row it links to and its tooltip text."""

    text: str  # as written, exponent included: "mSv", "s²"
    anchor: str  # the id of its row in docs/start/symbols.md: "sym-Sv"
    tooltip: str  # "milli- sievert · dose equivalent"


def _unbacktick(cell: str) -> str:
    return _BACKTICK.sub(r"\1", cell).strip()


def _bare(path: str) -> str:
    """The last segment of a `mod::Path`, as docs/start/symbols.md's generator strips it for the Marker column."""
    return path.rsplit("::", 1)[-1]


def _parse_tables(text: str) -> dict[str, list[list[str]]]:
    tables: dict[str, list[list[str]]] = {}
    for m in _TABLE.finditer(text):
        rows = []
        for line in m.group(2).splitlines():
            line = line.strip()
            if not line.startswith("|"):
                continue
            cells = [c.strip() for c in line.strip("|").split("|")]
            if all(re.fullmatch(r"-+", c) for c in cells):
                continue
            rows.append(cells)
        tables[m.group(1)] = rows[1:]  # drop the header row
    return tables


@dataclass(frozen=True)
class UnitEntry:
    marker: str  # bare: "Becquerel"
    prefixes: str  # "none", "SI", "SI and binary" or "SI from kilo up"


@dataclass(frozen=True)
class Catalog:
    units: dict[str, UnitEntry]
    folded: dict[str, str]  # symbol -> "kilo Meter"
    refused: dict[str, str]  # symbol -> "Candela, Cord"
    origins: dict[str, str]  # a written form, bare or quoted -> path ("Celsius")
    levels: dict[str, str]  # symbol -> bare marker ("Db")


def slugify_row(first_cell_text: str) -> str:
    """The id a row gets (`unit_markup.SymbolAnchors`) and the key this module looks it up by: the first cell's
    text, backticks and any `mod::` path stripped. Shared so an anchor always matches the symbol that targets it.
    """
    return _bare(_unbacktick(first_cell_text))


@functools.lru_cache(maxsize=1)
def load_catalog(path: Path = SYMBOLS_PAGE) -> Catalog:
    tables = _parse_tables(path.read_text())
    units = {}
    for symbol_cell, marker_cell, _feature, prefixes_cell in tables["units"]:
        symbol = _unbacktick(symbol_cell)
        marker = _unbacktick(marker_cell).split(" · ")[0]
        units[symbol] = UnitEntry(marker=_bare(marker), prefixes=_unbacktick(prefixes_cell))

    folded = {}
    for symbol_cell, _line, read_as_cell in tables["folded"]:
        folded[_unbacktick(symbol_cell)] = _unbacktick(read_as_cell)

    refused = {}
    for symbol_cell, reads_as_cell, _write in tables["refused"]:
        refused[_unbacktick(symbol_cell)] = _unbacktick(reads_as_cell)

    origins = {}
    for origin_cell, _feature, bare_cell, quoted_cell in tables["origins"]:
        path_name = _bare(_unbacktick(origin_cell))
        forms = [_unbacktick(bare_cell)]
        forms += [f.strip().strip('"') for f in _unbacktick(quoted_cell).split(",") if f.strip()]
        for form in forms:
            origins[form] = path_name

    levels = {}
    for symbol_cell, level_cell, *_rest in tables["levels"]:
        levels[_unbacktick(symbol_cell)] = _bare(_unbacktick(level_cell))

    return Catalog(units=units, folded=folded, refused=refused, origins=origins, levels=levels)


def _split_exponent(token: str) -> tuple[str, str]:
    m = re.search(r"\^-?\d+$", token)
    if m:
        return token[: m.start()], m.group(0)
    i = len(token)
    while i > 0 and token[i - 1] in _SUPERSCRIPT:
        i -= 1
    suffix = token[i:]
    if suffix and any(c != "⁻" for c in suffix) and i > 0:
        return token[:i], suffix
    return token, ""


def _tooltip(name: str, quantity: str | None) -> str:
    return f"{name} · {quantity}" if quantity else name


def _unit_tooltip(marker: str, prefix_name: str | None) -> str:
    info = UNIT_INFO.get(marker)
    if info is None or not info.name:
        raise UnitSymbolError(
            f"unit_catalog: no long name for marker {marker!r} in unit_info.json: give its unit! line a"
            " `long \"..\"` and run `just q-symbols`"
        )
    name = f"{prefix_name}- {info.name}" if prefix_name else info.name
    return _tooltip(name, info.quantity)


def resolve_base(base: str, catalog: Catalog) -> Factor:
    """One atomic token, no exponent: a bare symbol, a folded prefix+unit, an origin or a level."""
    if base in catalog.refused:
        raise UnitSymbolError(f"[[{base}]]: reads more than one way ({catalog.refused[base]}), write the marker")

    if base in catalog.units:
        entry = catalog.units[base]
        return Factor(text=base, anchor=f"sym-{base}", tooltip=_unit_tooltip(entry.marker, None))

    if base in catalog.folded:
        prefix_word, marker = catalog.folded[base].rsplit(" ", 1)
        return Factor(text=base, anchor=f"sym-{base}", tooltip=_unit_tooltip(marker, prefix_word))

    if base in catalog.origins:
        path_name = catalog.origins[base]
        info = ORIGIN_INFO.get(path_name)
        if info is None or not info.name:
            raise UnitSymbolError(f"unit_catalog: no entry for origin {path_name!r} in unit_info.json")
        return Factor(text=base, anchor=f"sym-{path_name}", tooltip=_tooltip(info.name, info.quantity))

    if base in catalog.levels:
        marker = catalog.levels[base]
        info = LEVEL_INFO.get(marker)
        if info is None or not info.name:
            raise UnitSymbolError(f"unit_catalog: no entry for level {marker!r} in unit_info.json")
        return Factor(text=base, anchor=f"sym-{base}", tooltip=_tooltip(info.name, info.quantity))

    # A prefix on a symbol that takes it: "keV" = kilo `ElectronVolt`, "mcd" = milli `Candela` (never milli `Cord`,
    # as `Cord` takes no prefix: only one candidate ever matches).
    candidates = []
    for prefix_symbol, prefix in PREFIX_INFO.items():
        if not base.startswith(prefix_symbol) or len(base) <= len(prefix_symbol):
            continue
        remainder = base[len(prefix_symbol) :]
        entry = catalog.units.get(remainder)
        if entry is None:
            continue
        allowed = {
            "none": False,
            "SI": prefix.kind == "si",
            "SI and binary": prefix.kind in ("si", "binary"),
            "SI from kilo up": prefix.kind == "si" and prefix.exp is not None and prefix.exp >= 3,
        }.get(entry.prefixes, False)
        if allowed:
            candidates.append((prefix, entry, remainder))

    if len(candidates) == 1:
        prefix, entry, remainder = candidates[0]
        return Factor(text=base, anchor=f"sym-{remainder}", tooltip=_unit_tooltip(entry.marker, prefix.name))
    if len(candidates) > 1:
        readings = ", ".join(f"{p.name} {e.marker}" for p, e, _ in candidates)
        raise UnitSymbolError(f"[[{base}]]: reads more than one way ({readings}), write the marker")

    raise UnitSymbolError(f"[[{base}]]: not a symbol q! reads (docs/start/symbols.md)")


def resolve_factor(token: str, catalog: Catalog) -> tuple[Factor, str]:
    """One compound factor with its exponent, already split off: the resolved base, and the exponent as written."""
    base, exponent = _split_exponent(token)
    factor = resolve_base(base, catalog)
    return factor, exponent


def resolve_expression(text: str, catalog: Catalog | None = None) -> list[tuple[str, object]]:
    """`[[..]]`'s content, parsed as q! reads it: a symbol, optionally prefixed, or a `/` and `·` compound with
    exponents. Returns a list of `("factor", Factor)` and `("text", str)` parts in reading order, for rendering;
    raises `UnitSymbolError` naming the exact token that failed.
    """
    catalog = catalog or load_catalog()
    if text.count("/") > 1:
        raise UnitSymbolError(f"[[{text}]]: more than one '/', write it as a marker")

    numerator, _, denominator = text.partition("/")
    parts: list[tuple[str, object]] = []
    for side_index, side in enumerate((numerator, denominator) if denominator else (numerator,)):
        if side_index == 1:
            parts.append(("text", "/"))
        factors = side.split("·")
        for i, token in enumerate(factors):
            if i > 0:
                parts.append(("text", "·"))
            if not token:
                raise UnitSymbolError(f"[[{text}]]: empty factor")
            factor, exponent = resolve_factor(token, catalog)
            parts.append(("factor", factor))
            if exponent:
                parts.append(("text", exponent))
    return parts
