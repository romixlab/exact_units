"""The long name and quantity of each unit, origin and level marker, for the tooltip of `[[symbol]]` and `\\unit{}`,
and the prefixes `unit_catalog` splits off a symbol.

All of it is read from `unit_info.json` beside this module, which `just q-symbols` generates (tests/q_symbols.rs,
the same walk of the source that writes q!'s table and docs/start/symbols.md): the long names and quantities of the
`unit!` lines, the origins of `src/point/origins.rs`, the level symbols, and the prefixes of `src/si_prefix.rs`.
`just test` fails while the file is stale. Nothing is curated here: a marker without a long name gets one on its
`unit!` line, in the crate.

Keys are the bare marker names (`Becquerel`, not `units::Becquerel`; `Celsius`; `Db`). A prefix's `kind` is "si" or
"binary"; `exp` is the power of ten an SI prefix carries, used to tell "SI from kilo up" (the tonne) apart from the
rest.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

UNIT_INFO_JSON = Path(__file__).resolve().parent / "unit_info.json"


@dataclass(frozen=True)
class PrefixEntry:
    name: str
    kind: str  # "si" or "binary"
    exp: int | None = None  # power of ten, SI prefixes only


@dataclass(frozen=True)
class Info:
    name: str | None  # None for a marker whose unit! line has no long name (one q!'s table doesn't read)
    quantity: str | None  # None for a counted thing or a level unit, which is a base of its own
    plural: str | None = None


def _infos(section: dict[str, dict]) -> dict[str, Info]:
    return {k: Info(name=v["long"], quantity=v["quantity"], plural=v["plural"]) for k, v in section.items()}


_DATA = json.loads(UNIT_INFO_JSON.read_text(encoding="utf-8"))

PREFIX_INFO: dict[str, PrefixEntry] = {
    symbol: PrefixEntry(p["name"], p["kind"], p["exp"]) for symbol, p in _DATA["prefixes"].items()
}
UNIT_INFO: dict[str, Info] = _infos(_DATA["units"])
ORIGIN_INFO: dict[str, Info] = _infos(_DATA["origins"])
LEVEL_INFO: dict[str, Info] = _infos(_DATA["levels"])
