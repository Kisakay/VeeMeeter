#!/usr/bin/env python3
"""VeeMeeter decorative kit — sparkles, doodles, pattern tiles, dividers,
social banner. Solid fills only.

    python3 branding/src/decor.py
"""
from __future__ import annotations

from pathlib import Path

import mascot as m

ROOT = Path(__file__).resolve().parent.parent
DECOR = ROOT / "decor"
SOCIAL = ROOT / "social"

INK, CREAM, WHITE = m.INK, m.CREAM, m.WHITE
NIGHT = "#1E1830"


def svg(inner: str, w: int, h: int, title: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-labelledby="t">'
        f'<title id="t">{title}</title>\n{inner}\n</svg>\n'
    )


def flat(s: str) -> str:
    """Drop the ink outline (for quiet background motifs)."""
    return s.replace(f'stroke="{INK}"', 'stroke="none"')


def star(x: float, y: float, r: float, c: str, outline: bool = True) -> str:
    import math

    pts = []
    for i in range(10):
        a = -math.pi / 2 + i * math.pi / 5
        rr = r if i % 2 == 0 else r * 0.5
        pts.append(f"{x + rr * math.cos(a):.1f} {y + rr * math.sin(a):.1f}")
    st = f'stroke="{INK}" stroke-width="4" stroke-linejoin="round"' if outline else ""
    return f'<path d="M{" L".join(pts)} Z" fill="{c}" {st}/>'


def wave(x: float, y: float, w: float, amp: float, n: int, c: str, sw: float = 6) -> str:
    step = w / n
    d = f"M{x} {y}"
    for i in range(n):
        d += f" q{step / 4:g} {-amp:g} {step / 2:g} 0 t{step / 2:g} 0"
    return f'<path d="{d}" fill="none" stroke="{c}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"/>'


def dot(x: float, y: float, r: float, c: str) -> str:
    return f'<circle cx="{x}" cy="{y}" r="{r}" fill="{c}"/>'


def pattern(bg: str, a: str, b: str, c: str, d: str) -> str:
    """160x160 seamless tile (motifs kept inside so edges match)."""
    return (
        f'<rect width="160" height="160" fill="{bg}"/>'
        + flat(m.sparkle(30, 30, 12, a))
        + flat(m.heart(110, 40, 9, b))
        + dot(80, 80, 4, c)
        + flat(star(40, 115, 10, d, False))
        + flat(m.sparkle(120, 120, 8, c))
        + dot(150, 85, 3, a)
        + dot(5, 75, 3, b)
    )


def divider(c1: str, c2: str) -> str:
    return (
        wave(20, 30, 520, 10, 8, c1)
        + m.heart(600, 30, 13, c2)
        + wave(660, 30, 520, 10, 8, c1)
        + m.sparkle(570, 24, 8, m.LEMON)
        + m.sparkle(632, 22, 7, m.MINT)
    )


def eq_bars(x: float, y: float) -> str:
    """Little VU/EQ doodle."""
    out = ""
    for i, (h, c) in enumerate(zip((30, 52, 70, 44, 24), (m.MINT, m.MINT, m.LEMON, m.MINT, m.SKY))):
        out += f'<rect x="{x + i * 20}" y="{y - h}" width="14" height="{h}" rx="7" fill="{c}" stroke="{INK}" stroke-width="4"/>'
    return out


def banner() -> str:
    W, H = 1280, 640
    import logo

    word = logo.wordmark(m.PINK, m.HAIR, INK, WHITE)
    happy = m.figure("happy", props=False)
    sticker = m.silhouette(happy, 30, WHITE) + happy
    s = (
        f'<rect width="{W}" height="{H}" fill="{m.HAIR}"/>'
        # big flat shapes
        f'<circle cx="1010" cy="340" r="250" fill="{m.HAIR_HI}"/>'
        f'<circle cx="1180" cy="90" r="90" fill="{m.PINK}"/>'
        f'<circle cx="80" cy="600" r="120" fill="{m.MINT}"/>'
        f'<rect x="56" y="88" width="660" height="464" rx="44" fill="{CREAM}" stroke="{INK}" stroke-width="7"/>'
        + f'<svg x="80" y="120" width="620" height="151" viewBox="0 0 820 200">{word}</svg>'
        + f'<text x="104" y="330" font-family="Nunito, Quicksand, \'Varela Round\', sans-serif" font-weight="800" font-size="38" fill="{INK}">A cute virtual audio mixer</text>'
        + f'<text x="104" y="378" font-family="Nunito, Quicksand, \'Varela Round\', sans-serif" font-weight="800" font-size="38" fill="{INK}">for Linux &amp; PipeWire.</text>'
        # tags
        + "".join(
            f'<rect x="{x}" y="420" width="{w}" height="52" rx="26" fill="{c}" stroke="{INK}" stroke-width="5"/>'
            f'<text x="{x + w / 2}" y="455" text-anchor="middle" font-family="Nunito, Quicksand, sans-serif" font-weight="800" font-size="25" fill="{INK}">{t}</text>'
            for x, w, c, t in ((104, 120, m.PINK, "Rust"), (240, 160, m.MINT, "PipeWire"), (416, 130, m.SKY, "NixOS"), (562, 110, m.LEMON, "egui"))
        )
        + wave(104, 515, 560, 8, 10, m.HAIR, 6)
        + f'<g transform="translate(1010 340) scale(1.05) translate(-256 -256)">{sticker}</g>'
        + m.sparkle(760, 110, 22, m.LEMON)
        + m.sparkle(1230, 560, 18, m.MINT)
        + m.heart(780, 560, 18, m.PINK)
        + m.note(1200, 230, m.SKY)
        + star(740, 300, 16, m.LEMON)
    )
    return s


UI = {
    "bg": "#1E1830", "panel": "#272040", "card": "#312852", "border": "#4A3F73",
    "text": "#F7F1FF", "muted": "#B8ADD6", "meter_bg": "#15111F", "green": "#7FE0A6",
    "yellow": "#FFD86B", "red": "#FF6B8B", "mute": "#FF5C7A", "a": "#8CCBFF", "b": "#B9A3F5",
    "star": "#FFE07A", "accent": "#FF8FB8",
}
FONT = "font-family=\"Nunito, Quicksand, 'Varela Round', sans-serif\" font-weight=\"800\""


def ui_text(x: float, y: float, t: str, size: int, c: str, anchor: str = "start") -> str:
    return f'<text x="{x}" y="{y}" {FONT} font-size="{size}" fill="{c}" text-anchor="{anchor}">{t}</text>'


def ui_button(x: float, y: float, w: float, label: str, on: bool, colour: str) -> str:
    fill = colour if on else UI["panel"]
    txt = INK if on else UI["muted"]
    return (
        f'<rect x="{x}" y="{y}" width="{w}" height="36" rx="12" fill="{fill}" stroke="{colour if on else UI["border"]}" stroke-width="3"/>'
        + ui_text(x + w / 2, y + 25, label, 17, txt, "middle")
    )


def ui_strip(x: float, name: str, sid: str, level: float, db: float, routes: tuple, muted: bool, default: bool, out: bool) -> str:
    y, w, h = 120, 200, 470
    s = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="22" fill="{UI["card"]}" stroke="{UI["border"]}" stroke-width="2"/>'
    s += ui_text(x + 20, y + 38, name, 20, UI["text"])
    s += f'<rect x="{x + 20}" y="{y + 52}" width="{12 + 11 * len(sid)}" height="24" rx="12" fill="{UI["panel"]}"/>' + ui_text(x + 26 + 5.5 * len(sid), y + 69, sid, 14, UI["muted"], "middle")
    if default:
        s += m.sparkle(x + w - 28, y + 30, 11, UI["star"]).replace(f'stroke="{INK}"', 'stroke="none"')
    # meters (two channels)
    top, bot = y + 96, y + 330
    full = bot - top
    for i in range(2):
        mx = x + 26 + i * 18
        s += f'<rect x="{mx}" y="{top}" width="12" height="{full}" rx="6" fill="{UI["meter_bg"]}"/>'
        lv = max(0.0, level - i * 0.06)
        if muted:
            lv = 0
        segs = ((0.0, 0.65, UI["green"]), (0.65, 0.88, UI["yellow"]), (0.88, 1.0, UI["red"]))
        for a, b, c in segs:
            if lv > a:
                hh = (min(lv, b) - a) * full
                s += f'<rect x="{mx}" y="{bot - b * full if lv >= b else bot - lv * full:.1f}" width="12" height="{hh:.1f}" rx="3" fill="{c}"/>'
    # fader
    fx = x + 120
    s += f'<rect x="{fx}" y="{top}" width="10" height="{full}" rx="5" fill="{UI["meter_bg"]}"/>'
    pos = bot - (db + 60) / 72 * full
    s += f'<rect x="{fx}" y="{pos:.1f}" width="10" height="{bot - pos:.1f}" rx="5" fill="{UI["b"] if not out else UI["a"]}"/>'
    s += f'<rect x="{fx - 17}" y="{pos - 13:.1f}" width="44" height="26" rx="10" fill="{UI["accent"]}" stroke="{INK}" stroke-width="3"/>'
    s += f'<path d="M{fx - 5} {pos:.1f} H{fx + 15}" stroke="{INK}" stroke-width="3" stroke-linecap="round"/>'
    s += ui_text(x + w / 2, y + 362, ("-inf" if db <= -60 else f"{db:+.1f}") + " dB", 16, UI["muted"], "middle")
    if not out:
        s += ui_button(x + 20, y + 378, 76, "A1", "a1" in routes, UI["a"])
        s += ui_button(x + 104, y + 378, 76, "B1", "b1" in routes, UI["b"])
    else:
        s += ui_text(x + w / 2, y + 402, "hardware" if sid.startswith("a") else "virtual mic", 15, UI["muted"], "middle")
    s += ui_button(x + 20, y + 422, 160, "MUTED" if muted else "Mute", muted, UI["mute"])
    return s


def mockup() -> str:
    W, H = 1240, 620
    s = f'<rect width="{W}" height="{H}" fill="{UI["bg"]}"/>'
    s += f'<rect width="{W}" height="76" fill="{UI["panel"]}"/>'
    import logo
    s += f'<svg x="20" y="12" width="52" height="52" viewBox="0 0 32 32">{logo.tiny(m.SKY)}</svg>'
    s += ui_text(86, 48, "VeeMeeter", 26, UI["text"])
    s += f'<rect x="{W - 230}" y="22" width="206" height="34" rx="17" fill="{UI["card"]}"/>' + f'<circle cx="{W - 208}" cy="39" r="7" fill="{UI["green"]}"/>' + ui_text(W - 192, 45, "daemon running", 16, UI["muted"])
    s += ui_text(40, 108, "INPUTS", 15, UI["muted"]) + ui_text(800, 108, "OUTPUTS", 15, UI["muted"])
    s += ui_strip(40, "Microphone", "hi1", 0.55, 0.0, ("b1",), False, False, False)
    s += ui_strip(276, "Desktop", "vi1", 0.78, -6.0, ("a1",), False, True, False)
    s += ui_strip(512, "Voice chat", "vi2", 0.2, 0.0, ("a1",), True, False, False)
    s += f'<rect x="752" y="130" width="4" height="450" rx="2" fill="{UI["border"]}"/>'
    s += ui_strip(800, "Speakers", "a1", 0.74, 0.0, (), False, False, True)
    s += ui_strip(1016, "Virtual Mic", "b1", 0.5, 0.0, (), False, True, True)
    return s


def main() -> None:
    DECOR.mkdir(parents=True, exist_ok=True)
    SOCIAL.mkdir(parents=True, exist_ok=True)
    files = {
        DECOR / "sparkle.svg": svg(m.sparkle(32, 32, 26, m.LEMON), 64, 64, "Sparkle"),
        DECOR / "sparkle-pink.svg": svg(m.sparkle(32, 32, 26, m.PINK), 64, 64, "Pink sparkle"),
        DECOR / "star.svg": svg(star(32, 34, 27, m.LEMON), 64, 64, "Star"),
        DECOR / "heart.svg": svg(m.heart(32, 32, 24, m.PINK), 64, 64, "Heart"),
        DECOR / "note.svg": svg(m.note(20, 18, m.MINT), 64, 64, "Music note"),
        DECOR / "sound-wave.svg": svg(wave(10, 32, 220, 18, 6, m.PINK, 8), 240, 64, "Sound wave"),
        DECOR / "eq-bars.svg": svg(eq_bars(8, 84), 112, 96, "Meter bars"),
        DECOR / "pattern-light.svg": svg(pattern(CREAM, m.PINK, m.HAIR, m.MINT, m.LEMON), 160, 160, "Pattern tile (light)"),
        DECOR / "pattern-dark.svg": svg(pattern(NIGHT, "#4A3F73", "#5B3FB8", "#3B3160", "#4A3F73"), 160, 160, "Pattern tile (dark)"),
        DECOR / "divider.svg": svg(divider(m.HAIR, m.PINK), 1200, 60, "Section divider"),
        DECOR / "divider-mint.svg": svg(divider(m.MINT, m.PINK), 1200, 60, "Section divider (mint)"),
        DECOR / "mute-badge.svg": svg(m.mute_badge(36, 36), 72, 72, "Muted"),
        SOCIAL / "banner.svg": svg(banner(), 1280, 640, "VeeMeeter social preview"),
        SOCIAL / "ui-mockup.svg": svg(mockup(), 1240, 620, "VeeMeeter mixer window (mockup)"),
    }
    for p, s in files.items():
        p.write_text(s)
    print("wrote", len(files), "files")


if __name__ == "__main__":
    main()
