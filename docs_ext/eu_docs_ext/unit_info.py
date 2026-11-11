"""What the tooltip of `[[symbol]]` and `\\unit{}` shows about each unit, origin and level marker, and the prefixes
`unit_catalog` splits off a symbol.

All of it is read from `unit_info.json` beside this module, which `just q-symbols` generates (tests/q_symbols.rs,
the same walk of the source that writes q!'s table and docs/start/symbols.md), from what the source already says:
a unit's symbol, long name and plural (its `unit!` line), its quantity (the line's `quantity`, else its dimension's
open root, else the dimension's coherent unit named after its quantity), its dimension in SI, its exact scale to
the coherent SI unit, the first sentence of its rustdoc, and the other markers of its quantity or dimension; each
origin's and level's name and docs; the prefixes of `src/si_prefix.rs`. `just test` fails while the file is stale.
Nothing is curated here: where the source says nothing, the field is `None` and the tooltip shows less.

Keys are the bare marker names (`Becquerel`, not `units::Becquerel`; `Celsius`; `Db`). A prefix's `kind` is "si" or
"binary"; `exp` is the power of ten an SI prefix carries, used to tell "SI from kilo up" (the tonne) apart from the
rest.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

UNIT_INFO_JSON = Path(__file__).resolve().parent / "unit_info.json"


@dataclass(frozen=True)
class PrefixEntry:
    name: str
    kind: str  # "si" or "binary"
    exp: int | None = None  # power of ten, SI prefixes only


@dataclass(frozen=True)
class Info:
    name: str | None  # the long name, None where the source has none
    quantity: str | None = None
    plural: str | None = None
    symbol: str | None = None
    dim: str | None = None  # in SI: "kg·m²·s⁻²"; None for dimension one
    scale: str | None = None  # the exact factor to the coherent SI unit: "0.0254", "π/180"; None for 1
    description: str | None = None  # the first sentence of the marker's rustdoc
    related: tuple[str, ...] = field(default_factory=tuple)  # other markers of the quantity, or of the dimension
    unit: str | None = None  # an origin's or a level's unit marker


def _info(v: dict) -> Info:
    return Info(
        name=v.get("long"),
        quantity=v.get("quantity"),
        plural=v.get("plural"),
        symbol=v.get("symbol"),
        dim=v.get("dim"),
        scale=v.get("scale"),
        description=v.get("description"),
        related=tuple(v.get("related", ())),
        unit=v.get("unit"),
    )


_DATA = json.loads(UNIT_INFO_JSON.read_text(encoding="utf-8"))

PREFIX_INFO: dict[str, PrefixEntry] = {
    symbol: PrefixEntry(p["name"], p["kind"], p["exp"]) for symbol, p in _DATA["prefixes"].items()
}
UNIT_INFO: dict[str, Info] = {k: _info(v) for k, v in _DATA["units"].items()}
ORIGIN_INFO: dict[str, Info] = {k: _info(v) for k, v in _DATA["origins"].items()}
LEVEL_INFO: dict[str, Info] = {k: _info(v) for k, v in _DATA["levels"].items()}
