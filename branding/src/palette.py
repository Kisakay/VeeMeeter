#!/usr/bin/env python3
"""Check branding/palette.toml contrast claims and write branding/palette.css.

    python3 branding/src/palette.py      # exits 1 if a stated ratio is wrong
"""
from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def lum(h: str) -> float:
    c = [int(h[i : i + 2], 16) / 255 for i in (1, 3, 5)]
    c = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def ratio(a: str, b: str) -> float:
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def lookup(pal: dict, name: str) -> str:
    if "." in name:
        node = pal
        for part in name.split("."):
            node = node[part]
        return node
    return pal["brand"][name]


def main() -> int:
    pal = tomllib.loads((ROOT / "palette.toml").read_text())
    bad = 0
    for c in pal["contrast"]:
        a, b = (lookup(pal, n) for n in c["pair"])
        r = ratio(a, b)
        need = {"AAA": 7.0, "AA": 4.5}[c["level"]]
        ok = abs(r - c["ratio"]) < 0.02 and r >= need
        bad += not ok
        print(f"{'ok ' if ok else 'BAD'} {c['pair'][0]:>24} on {c['pair'][1]:<20} {r:5.2f}:1 ({c['level']})")

    css = ["/* Generated from palette.toml by branding/src/palette.py — do not edit. */", ":root {"]
    for k, v in pal["brand"].items():
        css.append(f"  --vm-{k.replace('_', '-')}: {v};")
    for scheme in ("dark", "light"):
        for k, v in pal["ui"][scheme].items():
            css.append(f"  --vm-ui-{scheme}-{k.replace('_', '-')}: {v};")
    css.append("}")
    (ROOT / "palette.css").write_text("\n".join(css) + "\n")

    # swatch chips for the README tables (local, no external service)
    sw = ROOT / "swatches"
    sw.mkdir(exist_ok=True)
    for k, v in pal["brand"].items():
        (sw / f"{k}.svg").write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">'
            f'<rect x="1.5" y="1.5" width="21" height="21" rx="6" fill="{v}" stroke="#2B2140" stroke-width="2"/></svg>\n'
        )
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
