#!/usr/bin/env python3
"""Mimi, the VeeMeeter mascot — SVG generator.

Every pose is assembled from the same hand-drawn parts below (flat solid
fills, thick round ink outlines, cel shading = flat darker shapes). Run:

    python3 branding/src/mascot.py

and it rewrites branding/mascot/*.svg. Edit the parts, not the outputs.
"""
from __future__ import annotations

import re
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "mascot"

# Palette (mirrors branding/palette.toml [mascot]).
INK = "#2B2140"
SKIN = "#FFE6D5"
SKIN_SH = "#F7C9B3"
HAIR = "#B9A3F5"
HAIR_SH = "#9B83E8"
HAIR_HI = "#DCD1FC"
PINK = "#FF8FB8"
PINK_SH = "#EE6A9C"
MINT = "#7FE0C2"
MINT_SH = "#56C6A4"
LEMON = "#FFE07A"
SKY = "#8CCBFF"
CREAM = "#FFF7EE"
BLUSH = "#FFADC6"
IRIS = "#7B5CD6"
IRIS_DEEP = "#4E3596"
TIGHTS = "#4A3D6B"
GREY = "#CFC6E6"
RED = "#FF5C7A"
WHITE = "#FFFFFF"

W = 7  # outline width

def o(extra: str = "") -> str:
    """Outline attributes."""
    return f'stroke="{INK}" stroke-width="{W}" stroke-linejoin="round" stroke-linecap="round"{extra}'


def limb(points: str, colour: str, width: int = 30) -> str:
    """A rounded limb drawn as an outlined thick stroke."""
    return (
        f'<path d="{points}" fill="none" stroke="{INK}" stroke-width="{width + 2 * W}" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<path d="{points}" fill="none" stroke="{colour}" stroke-width="{width}" stroke-linecap="round" stroke-linejoin="round"/>'
    )


def hand(cx: float, cy: float, r: float = 15) -> str:
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{SKIN}" {o()}/>'


# ---------------------------------------------------------------- parts

def hair_back() -> str:
    return (
        f'<path d="M134 240 C124 140 186 92 256 92 C326 92 388 140 378 240 '
        f'L384 322 Q366 340 344 326 L338 296 L174 296 L168 326 Q146 340 128 322 Z" fill="{HAIR}" {o()}/>'
        # cel shade on the hanging locks
        f'<path d="M142 300 L146 322 Q156 328 164 322 L168 300 Z" fill="{HAIR_SH}"/>'
        f'<path d="M370 300 L366 322 Q356 328 348 322 L344 300 Z" fill="{HAIR_SH}"/>'
    )


def legs() -> str:
    return (
        f'<rect x="214" y="420" width="32" height="46" rx="14" fill="{TIGHTS}" {o()}/>'
        f'<rect x="266" y="420" width="32" height="46" rx="14" fill="{TIGHTS}" {o()}/>'
        f'<path d="M204 476 Q204 456 230 456 Q252 456 252 470 Q252 482 236 482 L214 482 Q204 482 204 476 Z" fill="{PINK}" {o()}/>'
        f'<path d="M308 476 Q308 456 282 456 Q260 456 260 470 Q260 482 276 482 L298 482 Q308 482 308 476 Z" fill="{PINK}" {o()}/>'
    )


def body() -> str:
    return (
        # hoodie
        f'<path d="M204 318 Q170 332 168 392 L166 428 Q166 440 178 440 L334 440 Q346 440 346 428 '
        f'L344 392 Q342 332 308 318 Z" fill="{MINT}" {o()}/>'
        # cel shade + pocket
        f'<path d="M178 436 L174 404 Q206 414 256 414 Q306 414 338 404 L334 436 Z" fill="{MINT_SH}"/>'
        f'<path d="M214 404 Q256 416 298 404 L294 430 L218 430 Z" fill="{MINT}" {o()}/>'
        # V emblem
        f'<path d="M238 352 L256 380 L274 352" fill="none" stroke="{INK}" stroke-width="17" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<path d="M238 352 L256 380 L274 352" fill="none" stroke="{PINK}" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>'
        # drawstrings
        f'<path d="M236 324 L232 344" fill="none" stroke="{INK}" stroke-width="4" stroke-linecap="round"/>'
        f'<path d="M276 324 L280 344" fill="none" stroke="{INK}" stroke-width="4" stroke-linecap="round"/>'
        f'<circle cx="231" cy="348" r="5" fill="{PINK}" stroke="{INK}" stroke-width="3"/>'
        f'<circle cx="281" cy="348" r="5" fill="{PINK}" stroke="{INK}" stroke-width="3"/>'
    )


def face() -> str:
    return (
        f'<ellipse cx="256" cy="232" rx="106" ry="94" fill="{SKIN}" {o()}/>'
        # bangs shadow on the forehead
        f'<path d="M160 214 Q256 250 352 214 L352 196 L160 196 Z" fill="{SKIN_SH}"/>'
    )


def bangs() -> str:
    return (
        f'<path d="M146 246 C134 150 192 110 256 110 C320 110 378 150 366 246 '
        f'Q352 226 346 196 Q328 220 300 224 Q302 200 290 182 Q268 214 236 220 '
        f'Q238 198 224 184 Q204 214 170 214 Q160 230 146 246 Z" fill="{HAIR}" {o()}/>'
        # highlight band (flat)
        f'<path d="M188 150 Q220 130 254 130" fill="none" stroke="{HAIR_HI}" stroke-width="9" stroke-linecap="round"/>'
        f'<path d="M276 132 Q298 134 314 144" fill="none" stroke="{HAIR_HI}" stroke-width="9" stroke-linecap="round"/>'
    )


def ahoge(kind: str = "wave") -> str:
    d = {
        "wave": "M232 100 C222 74 248 68 240 48 C234 34 248 22 264 28",
        "droop": "M232 100 C222 78 214 76 204 80 C194 84 192 96 198 104",
        "spike": "M236 100 L230 66 L246 74 L242 36",
        "curl": "M232 100 C222 74 248 68 240 48 C234 34 252 24 262 36 C268 44 258 52 252 46",
    }[kind]
    return (
        f'<path d="{d}" fill="none" stroke="{INK}" stroke-width="17" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<path d="{d}" fill="none" stroke="{HAIR}" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>'
    )


def cup(levels: tuple[str, str, str], heights: tuple[int, int, int]) -> str:
    """Left ear cup; the right one is the same group mirrored."""
    bars = ""
    for i, (c, h) in enumerate(zip(levels, heights)):
        x = 114 + i * 11
        bars += f'<rect x="{x}" y="{272 - h}" width="8" height="{h}" rx="4" fill="{c}" stroke="{INK}" stroke-width="2.5"/>'
    return (
        f'<rect x="142" y="200" width="28" height="84" rx="12" fill="{CREAM}" {o()}/>'
        f'<rect x="100" y="190" width="56" height="104" rx="24" fill="{PINK}" {o()}/>'
        f'<path d="M104 262 Q104 290 126 290 L130 290 Q108 286 108 262 Z" fill="{PINK_SH}"/>'
        f'<rect x="109" y="210" width="38" height="68" rx="12" fill="{INK}"/>'
        + bars
    )


def headphones(meter: str = "idle") -> str:
    sets = {
        "idle": ((MINT, MINT, LEMON), (22, 34, 46)),
        "peak": ((MINT, LEMON, RED), (40, 50, 58)),
        "off": ((GREY, GREY, GREY), (8, 8, 8)),
        "low": ((MINT, MINT, MINT), (14, 24, 18)),
    }
    lv, hs = sets[meter]
    band = "M140 232 C126 34 386 34 372 232"
    left = cup(lv, hs)
    return (
        f'<path d="{band}" fill="none" stroke="{INK}" stroke-width="36" stroke-linecap="round"/>'
        f'<path d="{band}" fill="none" stroke="{PINK}" stroke-width="22" stroke-linecap="round"/>'
        f'<path d="M168 112 Q210 80 256 80" fill="none" stroke="{WHITE}" stroke-width="5" stroke-linecap="round"/>'
        # fader knob riding on the band
        f'<rect x="276" y="70" width="44" height="26" rx="10" fill="{MINT}" {o()} transform="rotate(10 298 83)"/>'
        f'<path d="M298 76 L298 90" stroke="{INK}" stroke-width="4" stroke-linecap="round" transform="rotate(10 298 83)"/>'
        + f"<g>{left}</g>"
        + f'<g transform="translate(512 0) scale(-1 1)">{left}</g>'
    )


def blush(lines: bool = True) -> str:
    s = (
        f'<ellipse cx="184" cy="290" rx="18" ry="9" fill="{BLUSH}"/>'
        f'<ellipse cx="328" cy="290" rx="18" ry="9" fill="{BLUSH}"/>'
    )
    if lines:
        for cx in (184, 328):
            for dx in (-8, 0, 8):
                s += f'<path d="M{cx + dx + 3} {286} L{cx + dx - 2} {294}" stroke="{PINK_SH}" stroke-width="2.5" stroke-linecap="round"/>'
    return s


def eye_open(cx: float, cy: float, look: tuple[float, float] = (0, 0), outer: int = -1) -> str:
    lx, ly = look
    return (
        f'<ellipse cx="{cx}" cy="{cy}" rx="19" ry="26" fill="{INK}"/>'
        f'<ellipse cx="{cx + lx}" cy="{cy + 6 + ly}" rx="12" ry="14" fill="{IRIS}"/>'
        f'<ellipse cx="{cx + lx}" cy="{cy + 12 + ly}" rx="8" ry="6" fill="{IRIS_DEEP}"/>'
        f'<circle cx="{cx + 6 + lx}" cy="{cy - 9 + ly}" r="7" fill="{WHITE}"/>'
        f'<circle cx="{cx - 6 + lx}" cy="{cy + 9 + ly}" r="3.2" fill="{WHITE}"/>'
        f'<path d="M{cx + outer * 16} {cy - 12} Q{cx + outer * 22} {cy - 14} {cx + outer * 25} {cy - 20}" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>'
    )


def eyes(kind: str = "open") -> str:
    L, R, Y = 210, 302, 254
    if kind == "open":
        return eye_open(L, Y, outer=-1) + eye_open(R, Y, outer=1)
    if kind == "look":  # thinking: glancing up and to the side
        return eye_open(L, Y, (4, -4), -1) + eye_open(R, Y, (4, -4), 1)
    if kind == "happy":  # ^ ^
        return "".join(
            f'<path d="M{x - 18} {Y + 6} Q{x} {Y - 22} {x + 18} {Y + 6}" fill="none" stroke="{INK}" stroke-width="8" stroke-linecap="round"/>'
            for x in (L, R)
        )
    if kind == "sleepy":  # - - with lashes
        return "".join(
            f'<path d="M{x - 18} {Y + 2} Q{x} {Y + 16} {x + 18} {Y + 2}" fill="none" stroke="{INK}" stroke-width="7" stroke-linecap="round"/>'
            f'<path d="M{x + s * 18} {Y + 2} L{x + s * 25} {Y - 3}" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>'
            for x, s in ((L, -1), (R, 1))
        )
    if kind == "wide":  # surprised: big white eyes, tiny pupils
        return "".join(
            f'<ellipse cx="{x}" cy="{Y}" rx="19" ry="24" fill="{WHITE}" stroke="{INK}" stroke-width="6"/>'
            f'<circle cx="{x}" cy="{Y + 2}" r="7" fill="{INK}"/>'
            f'<circle cx="{x + 3}" cy="{Y - 1}" r="2.4" fill="{WHITE}"/>'
            for x in (L, R)
        )
    if kind == "wink":
        return eye_open(L, Y, outer=-1) + (
            f'<path d="M{R - 18} {Y + 4} Q{R} {Y - 16} {R + 18} {Y + 4}" fill="none" stroke="{INK}" stroke-width="8" stroke-linecap="round"/>'
        )
    raise ValueError(kind)


def mouth(kind: str = "cat") -> str:
    if kind == "cat":
        return f'<path d="M242 296 Q249 305 256 297 Q263 305 270 296" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>'
    if kind == "open":
        return (
            f'<path d="M238 292 Q256 296 274 292 Q272 316 256 316 Q240 316 238 292 Z" fill="{IRIS_DEEP}" stroke="{INK}" stroke-width="5" stroke-linejoin="round"/>'
            + f'<path d="M246 309 Q256 302 266 309 Q262 315 256 315 Q250 315 246 309 Z" fill="{PINK}"/>'
        )
    if kind == "o":
        return f'<ellipse cx="256" cy="303" rx="9" ry="11" fill="{IRIS_DEEP}" stroke="{INK}" stroke-width="5"/>'
    if kind == "small-o":
        return f'<ellipse cx="256" cy="300" rx="5" ry="5" fill="{IRIS_DEEP}" stroke="{INK}" stroke-width="4"/>'
    if kind == "wavy":
        return f'<path d="M242 300 Q248 294 254 300 Q260 306 266 300 Q270 296 272 298" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>'
    raise ValueError(kind)


# ------------------------------------------------------------ props

def sparkle(x: float, y: float, s: float, c: str) -> str:
    d = (
        f"M{x} {y - s} Q{x + s * 0.18} {y - s * 0.18} {x + s} {y} "
        f"Q{x + s * 0.18} {y + s * 0.18} {x} {y + s} Q{x - s * 0.18} {y + s * 0.18} {x - s} {y} "
        f"Q{x - s * 0.18} {y - s * 0.18} {x} {y - s} Z"
    )
    return f'<path d="{d}" fill="{c}" stroke="{INK}" stroke-width="4" stroke-linejoin="round"/>'


def heart(x: float, y: float, s: float, c: str) -> str:
    d = (
        f"M{x} {y + s * 0.9} C{x - s * 1.4} {y} {x - s * 0.9} {y - s * 1.0} {x} {y - s * 0.35} "
        f"C{x + s * 0.9} {y - s * 1.0} {x + s * 1.4} {y} {x} {y + s * 0.9} Z"
    )
    return f'<path d="{d}" fill="{c}" stroke="{INK}" stroke-width="4" stroke-linejoin="round"/>'


def note(x: float, y: float, c: str) -> str:
    return (
        f'<path d="M{x + 12} {y + 26} L{x + 12} {y - 6} Q{x + 22} {y - 2} {x + 28} {y + 8}" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<ellipse cx="{x + 4}" cy="{y + 27}" rx="10" ry="8" fill="{c}" stroke="{INK}" stroke-width="4" transform="rotate(-20 {x + 4} {y + 27})"/>'
    )


def motion(x: float, y: float) -> str:
    return "".join(
        f'<path d="{d}" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>'
        for d in (
            f"M{x} {y} Q{x + 10} {y - 14} {x + 6} {y - 30}",
            f"M{x + 16} {y + 8} Q{x + 28} {y - 4} {x + 26} {y - 18}",
        )
    )


def zzz() -> str:
    out = ""
    for x, y, s in ((372, 120, 22), (408, 84, 16), (434, 58, 11)):
        d = f"M{x} {y} L{x + s} {y} L{x} {y + s} L{x + s} {y + s}"
        out += f'<path d="{d}" fill="none" stroke="{INK}" stroke-width="{s / 3.4 + 4:.1f}" stroke-linecap="round" stroke-linejoin="round"/>'
        out += f'<path d="{d}" fill="none" stroke="{SKY}" stroke-width="{s / 3.4:.1f}" stroke-linecap="round" stroke-linejoin="round"/>'
    return out


def mute_badge(x: float, y: float) -> str:
    return (
        f'<circle cx="{x}" cy="{y}" r="26" fill="{CREAM}" {o()}/>'
        f'<path d="M{x - 14} {y - 6} L{x - 7} {y - 6} L{x + 2} {y - 14} L{x + 2} {y + 14} L{x - 7} {y + 6} L{x - 14} {y + 6} Z" fill="{INK}" stroke="{INK}" stroke-width="3" stroke-linejoin="round"/>'
        f'<path d="M{x + 7} {y - 6} L{x + 15} {y + 6} M{x + 15} {y - 6} L{x + 7} {y + 6}" stroke="{RED}" stroke-width="4.5" stroke-linecap="round"/>'
    )


def sweat(x: float, y: float) -> str:
    return f'<path d="M{x} {y - 18} Q{x + 14} {y + 2} {x + 10} {y + 10} Q{x} {y + 20} {x - 10} {y + 10} Q{x - 14} {y + 2} {x} {y - 18} Z" fill="{SKY}" stroke="{INK}" stroke-width="5" stroke-linejoin="round"/>'


def bubble(x: float, y: float, glyph: str) -> str:
    """Speech bubble with '?' or '!'."""
    g = {
        "?": f'<path d="M{x - 9} {y - 8} Q{x - 9} {y - 20} {x} {y - 20} Q{x + 10} {y - 20} {x + 10} {y - 10} Q{x + 10} {y - 3} {x} {y + 1} L{x} {y + 5}" fill="none" stroke="{INK}" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<circle cx="{x}" cy="{y + 15}" r="4" fill="{INK}"/>',
        "!": f'<path d="M{x} {y - 20} L{x} {y + 3}" stroke="{INK}" stroke-width="8" stroke-linecap="round"/>'
        f'<circle cx="{x}" cy="{y + 15}" r="4.5" fill="{INK}"/>',
    }[glyph]
    return (
        f'<path d="M{x - 30} {y + 26} Q{x - 40} {y + 40} {x - 46} {y + 44} Q{x - 26} {y + 42} {x - 18} {y + 32} Z" fill="{CREAM}" {o()}/>'
        f'<circle cx="{x}" cy="{y}" r="34" fill="{CREAM}" {o()}/>'
        f'<path d="M{x - 30} {y + 26} Q{x - 36} {y + 36} {x - 40} {y + 40} Q{x - 26} {y + 36} {x - 18} {y + 30} Z" fill="{CREAM}"/>'
        + g
    )


# ------------------------------------------------------------- poses

def arms(kind: str) -> str:
    L_rest = limb("M190 346 Q168 372 164 404", MINT) + hand(164, 410)
    R_rest = limb("M322 346 Q344 372 348 404", MINT) + hand(348, 410)
    if kind == "wave":
        return L_rest, limb("M324 344 Q372 338 398 300", MINT) + hand(404, 290, 16)
    if kind == "cheer":
        return (
            limb("M188 344 Q140 330 116 300", MINT) + hand(110, 290, 16),
            limb("M324 344 Q372 330 396 300", MINT) + hand(402, 290, 16),
        )
    if kind == "rest":
        return L_rest, R_rest
    if kind == "chin":
        return L_rest, limb("M324 350 Q334 386 300 378 Q286 372 282 346", MINT, 28) + hand(280, 330, 15)
    if kind == "shock":
        return (
            limb("M190 346 Q154 352 140 384", MINT) + hand(136, 392, 15),
            limb("M322 346 Q358 352 372 384", MINT) + hand(376, 392, 15),
        )
    raise ValueError(kind)


POSES = {
    # name: (arms, eyes, mouth, ahoge, meter, extras)
    "wave": ("wave", "open", "cat", "wave", "idle", "wave"),
    "happy": ("cheer", "happy", "open", "curl", "peak", "happy"),
    "sleepy": ("rest", "sleepy", "small-o", "droop", "off", "sleepy"),
    "surprised": ("shock", "wide", "o", "spike", "low", "surprised"),
    "thinking": ("chin", "look", "wavy", "wave", "idle", "thinking"),
    "wink": ("wave", "wink", "cat", "curl", "idle", "wave"),
}


def extras(kind: str) -> tuple[str, str]:
    """(behind, front) decorations."""
    if kind == "wave":
        return "", motion(424, 270) + sparkle(96, 120, 18, LEMON) + sparkle(420, 160, 12, PINK)
    if kind == "happy":
        return "", (
            sparkle(72, 196, 20, LEMON) + sparkle(444, 196, 20, LEMON)
            + heart(92, 110, 18, PINK) + heart(424, 104, 14, PINK)
            + note(54, 300, MINT) + note(432, 292, SKY)
        )
    if kind == "sleepy":
        return "", zzz() + mute_badge(110, 108)
    if kind == "surprised":
        return "", sweat(392, 156) + bubble(420, 92, "!")
    if kind == "thinking":
        return "", bubble(420, 96, "?") + sparkle(84, 140, 12, SKY)
    raise ValueError(kind)


def figure(pose: str, props: bool = True) -> str:
    a, e, m, h, meter, x = POSES[pose]
    arm_l, arm_r = arms(a)
    behind, front = extras(x) if props else ("", "")
    return (
        behind
        + hair_back()
        + legs()
        + body()
        + arm_l
        + face()
        + eyes(e)
        + blush(e != "wide")
        + mouth(m)
        + ahoge(h)
        + bangs()
        + headphones(meter)
        + arm_r
        + front
    )


def svg(inner: str, title: str, size: int = 512) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="{size}" height="{size}" role="img" aria-labelledby="t">'
        f"<title id=\"t\">{title}</title>\n{inner}\n</svg>\n"
    )


def silhouette(inner: str, grow: int = 22, colour: str = WHITE) -> str:
    """Every shape repainted in one colour with a fat stroke = sticker border."""
    s = re.sub(r'fill="(?!none)[^"]*"', f'fill="{colour}"', inner)
    s = re.sub(r'stroke="[^"]*"', f'stroke="{colour}"', s)
    s = re.sub(r'stroke-width="([\d.]+)"', lambda m: f'stroke-width="{float(m.group(1)) + grow:g}"', s)
    # filled shapes without an outline still need to grow
    s = re.sub(r"<(ellipse|circle|rect|path)((?:(?!stroke=)[^>])*?)/>", rf'<\1\2 stroke="{colour}" stroke-width="{grow}" stroke-linejoin="round"/>', s)
    return s


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    titles = {
        "wave": "Mimi waving hello",
        "happy": "Mimi cheering, meters peaking",
        "sleepy": "Mimi asleep, muted",
        "surprised": "Mimi surprised",
        "thinking": "Mimi thinking",
        "wink": "Mimi winking",
    }
    for pose, title in titles.items():
        (OUT / f"mimi-{pose}.svg").write_text(svg(figure(pose), title))
    # sticker variants: white die-cut border + soft ink drop line
    for pose in ("wave", "happy", "wink"):
        art = figure(pose, props=False)
        sticker = (
            '<g transform="translate(256 256) scale(0.9) translate(-256 -256)">'
            + f'<g transform="translate(0 8)">{silhouette(art, 34, HAIR_HI)}</g>'
            + silhouette(art, 34, WHITE)
            + art
            + "</g>"
        )
        (OUT / f"mimi-{pose}-sticker.svg").write_text(svg(sticker, titles[pose] + " (sticker)"))
    # head only, used by the app icon and avatar
    print("wrote", sorted(p.name for p in OUT.glob("*.svg")))


if __name__ == "__main__":
    main()
