#!/usr/bin/env python3
"""VeeMeeter logo kit — SVG generator (app icon, marks, wordmarks, lockups).

    python3 branding/src/logo.py

Rewrites branding/logo/*.svg, branding/favicon.svg and assets/veemeeter.svg.
The wordmark is hand-built bubble lettering (round monoline strokes), so the
files never depend on an installed font.
"""
from __future__ import annotations

import re
from pathlib import Path

import mascot as m

ROOT = Path(__file__).resolve().parent.parent  # branding/
LOGO = ROOT / "logo"
APP_ICON = ROOT.parent / "assets" / "veemeeter.svg"

INK, CREAM, WHITE = m.INK, m.CREAM, m.WHITE
DARK_BG = "#312852"  # ui.dark.card: stands out on the #1E1830 page


# ------------------------------------------------------------------ mark

def head(simple: bool = False) -> str:
    """Mimi's head (no body). `simple` drops details invisible at 16-32 px."""
    parts = (
        m.hair_back()
        + m.face()
        + m.eyes("open")
        + m.blush(lines=not simple)
        + m.mouth("cat")
        + m.ahoge("wave")
        + m.bangs()
        + m.headphones("idle")
    )
    if simple:
        # thicker outlines, no hair shine, no lashes
        parts = parts.replace(f'stroke-width="{m.W}"', 'stroke-width="11"')
        parts = re.sub(r'<path d="[^"]*" fill="none" stroke="#DCD1FC"[^>]*/>', "", parts)
    return parts


def mark(bg: str, simple: bool = False, rx: int = 116) -> str:
    """Square app icon: rounded tile + Mimi's head, centred."""
    return (
        f'<rect x="8" y="8" width="496" height="496" rx="{rx}" fill="{bg}"/>'
        f'<g transform="translate(256 270) scale(1.2) translate(-256 -182)">{head(simple)}</g>'
    )


DARK_SET = {m.INK, m.IRIS, m.IRIS_DEEP, m.TIGHTS, m.PINK_SH}


def mono(svg_inner: str, ink: str, paper: str) -> str:
    """Two-value version: dark parts -> ink, everything else -> paper."""

    def sub(mt: re.Match) -> str:
        attr, val = mt.group(1), mt.group(2)
        if val == "none":
            return mt.group(0)
        return f'{attr}="{ink if val.upper() in DARK_SET else paper}"'

    return re.sub(r'(fill|stroke)="([^"]*)"', sub, svg_inner)


def tiny(bg: str) -> str:
    """Pixel-hinted 32-grid head for 16-48 px (favicon, tray, small icons)."""
    P, PS, H, SK, I = m.PINK, m.PINK_SH, m.HAIR, m.SKIN, INK
    return (
        f'<rect x="0" y="0" width="32" height="32" rx="8" fill="{bg}"/>'
        # hair
        f'<path d="M5.5 19 C5 9.5 10 5.5 16 5.5 C22 5.5 27 9.5 26.5 19 L26.5 25 L5.5 25 Z" fill="{H}" stroke="{I}" stroke-width="1.4" stroke-linejoin="round"/>'
        # face
        f'<ellipse cx="16" cy="20.5" rx="8.6" ry="7.2" fill="{SK}" stroke="{I}" stroke-width="1.4"/>'
        # bangs
        f'<path d="M7.4 19 C7 12 11 9.2 16 9.2 C21 9.2 25 12 24.6 19 Q23 17 22.6 15 Q20 17 17.5 16.4 Q17 14.6 16 13.6 Q14 16.4 10.6 16.6 Q9 17.4 7.4 19 Z" fill="{H}" stroke="{I}" stroke-width="1.2" stroke-linejoin="round"/>'
        # band + cups
        f'<path d="M5.5 18 C4.5 1.5 27.5 1.5 26.5 18" fill="none" stroke="{I}" stroke-width="4.4" stroke-linecap="round"/>'
        f'<path d="M5.5 18 C4.5 1.5 27.5 1.5 26.5 18" fill="none" stroke="{P}" stroke-width="2.2" stroke-linecap="round"/>'
        f'<rect x="2.5" y="14" width="5.6" height="10" rx="2.4" fill="{P}" stroke="{I}" stroke-width="1.4"/>'
        f'<rect x="23.9" y="14" width="5.6" height="10" rx="2.4" fill="{P}" stroke="{I}" stroke-width="1.4"/>'
        f'<rect x="4.2" y="17" width="2" height="5" rx="1" fill="{m.MINT}"/>'
        f'<rect x="25.8" y="17" width="2" height="5" rx="1" fill="{m.MINT}"/>'
        # eyes + blush
        f'<ellipse cx="12.6" cy="20.6" rx="1.7" ry="2.3" fill="{I}"/>'
        f'<ellipse cx="19.4" cy="20.6" rx="1.7" ry="2.3" fill="{I}"/>'
        f'<circle cx="13.1" cy="19.7" r="0.7" fill="{WHITE}"/>'
        f'<circle cx="19.9" cy="19.7" r="0.7" fill="{WHITE}"/>'
        f'<ellipse cx="10.4" cy="23.6" rx="1.5" ry="0.9" fill="{m.BLUSH}"/>'
        f'<ellipse cx="21.6" cy="23.6" rx="1.5" ry="0.9" fill="{m.BLUSH}"/>'
        f'<path d="M15 24 Q15.5 24.8 16 24.2 Q16.5 24.8 17 24" fill="none" stroke="{I}" stroke-width="0.8" stroke-linecap="round"/>'
    )


# -------------------------------------------------------------- wordmark

def letters() -> list[tuple[str, int, str]]:
    """(path at x=0, advance, group) for V e e M e e t e r."""
    e = "M2 120 H59 A29 30 0 1 0 51 141"
    return [
        ("M0 50 L32 148 L64 50", 64, "vee"),
        (e, 60, "vee"),
        (e, 60, "vee"),
        ("M0 148 V54 L36 108 L72 54 V148", 72, "meeter"),
        (e, 60, "meeter"),
        (e, 60, "meeter"),
        ("M14 58 V126 Q14 148 38 148 M0 92 H34", 40, "meeter"),
        (e, 60, "meeter"),
        ("M0 148 V92 M0 124 Q0 90 36 92", 38, "meeter"),
    ]


def shift(path: str, dx: float) -> str:
    """Translate an absolute M/L/H/V/Q/A path horizontally."""
    out, cmd, idx = [], "", 0
    for tok in re.findall(r"[A-Za-z]|-?[\d.]+", path):
        if tok.isalpha():
            cmd, idx = tok, 0
            out.append(tok)
            continue
        v = float(tok)
        if cmd == "H":
            v += dx
        elif cmd in "MLQ" and idx % 2 == 0:
            v += dx
        elif cmd == "A" and idx % 7 == 5:
            v += dx
        idx += 1
        out.append(f"{v:g}")
    return " ".join(out)


GAP = 30
X0 = 34
WORD_W = 820
WORD_H = 200


def word_paths() -> list[tuple[str, str]]:
    x, out = X0, []
    for d, adv, grp in letters():
        out.append((shift(d, x), grp))
        x += adv + GAP
    return out


def wordmark(vee: str, meeter: str, outline: str | None, halo: str | None, core: int = 22) -> str:
    paths = word_paths()
    s = ""
    if halo:
        s += "".join(f'<path d="{d}" fill="none" stroke="{halo}" stroke-width="{core + 34}" stroke-linecap="round" stroke-linejoin="round"/>' for d, _ in paths)
    if outline:
        s += "".join(f'<path d="{d}" fill="none" stroke="{outline}" stroke-width="{core + 16}" stroke-linecap="round" stroke-linejoin="round"/>' for d, _ in paths)
    s += "".join(
        f'<path d="{d}" fill="none" stroke="{vee if g == "vee" else meeter}" stroke-width="{core}" stroke-linecap="round" stroke-linejoin="round"/>'
        for d, g in paths
    )
    if outline:  # tiny shine on the V, the brand's "sparkle"
        s += m.sparkle(X0 + 690 + 52, 46, 16, m.LEMON).replace(f'stroke="{INK}"', f'stroke="{outline}"')
    return s


# ------------------------------------------------------------------ files

def svg(inner: str, w: int, h: int, title: str, vb: str | None = None) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb or f"0 0 {w} {h}"}" width="{w}" height="{h}" role="img" aria-labelledby="t">'
        f'<title id="t">{title}</title>\n{inner}\n</svg>\n'
    )


def nested(inner: str, x: float, y: float, w: float, h: float, vb: str) -> str:
    return f'<svg x="{x}" y="{y}" width="{w}" height="{h}" viewBox="{vb}">{inner}</svg>'


def main() -> None:
    LOGO.mkdir(parents=True, exist_ok=True)
    files: dict[Path, str] = {}

    icon = mark(m.SKY)
    files[APP_ICON] = svg(icon, 512, 512, "VeeMeeter")
    files[LOGO / "mark.svg"] = svg(icon, 512, 512, "VeeMeeter mark")
    files[LOGO / "mark-dark.svg"] = svg(mark(DARK_BG), 512, 512, "VeeMeeter mark (dark)")
    files[LOGO / "mark-small.svg"] = svg(tiny(m.SKY), 32, 32, "VeeMeeter mark (16-48 px)")
    files[ROOT / "favicon.svg"] = files[LOGO / "mark-small.svg"]
    files[LOGO / "mark-mono.svg"] = svg(mono(mark(WHITE, simple=True), INK, WHITE), 512, 512, "VeeMeeter mark (monochrome)")
    files[LOGO / "mark-mono-inverse.svg"] = svg(mono(mark(WHITE, simple=True), CREAM, INK), 512, 512, "VeeMeeter mark (monochrome, inverse)")

    word_light = wordmark(m.PINK, m.HAIR, INK, None)
    word_dark = wordmark(m.PINK, m.HAIR, INK, CREAM)
    files[LOGO / "wordmark.svg"] = svg(word_light, WORD_W, WORD_H, "VeeMeeter")
    files[LOGO / "wordmark-dark.svg"] = svg(word_dark, WORD_W, WORD_H, "VeeMeeter (for dark backgrounds)")
    files[LOGO / "wordmark-mono.svg"] = svg(wordmark(INK, INK, None, None, 24), WORD_W, WORD_H, "VeeMeeter (monochrome)")
    files[LOGO / "wordmark-mono-white.svg"] = svg(wordmark(WHITE, WHITE, None, None, 24), WORD_W, WORD_H, "VeeMeeter (monochrome white)")

    # horizontal lockups: mark + wordmark, 1100 x 240
    def lockup(icon_inner: str, word: str) -> str:
        return nested(icon_inner, 10, 10, 220, 220, "0 0 512 512") + nested(word, 250, 20, 840, 205, f"0 0 {WORD_W} {WORD_H}")

    files[LOGO / "logo-horizontal.svg"] = svg(lockup(icon, word_light), 1100, 240, "VeeMeeter logo")
    files[LOGO / "logo-horizontal-dark.svg"] = svg(lockup(mark(DARK_BG), word_dark), 1100, 240, "VeeMeeter logo (dark)")
    # stacked: mark above wordmark, 820 x 760
    stacked = nested(icon, 210, 10, 400, 400, "0 0 512 512") + nested(word_light, 0, 470, WORD_W, WORD_H, f"0 0 {WORD_W} {WORD_H}")
    files[LOGO / "logo-stacked.svg"] = svg(stacked, 820, 690, "VeeMeeter logo (stacked)")

    for p, s in files.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(s)
    print("wrote", len(files), "files")


if __name__ == "__main__":
    main()
