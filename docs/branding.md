# Branding kit

Kawaii, anime-flavoured, **solid colours only**. Every asset is hand-authored
SVG living in [`branding/`](https://github.com/Kisakay/VeeMeeter/tree/main/branding);
the full guide is its
[README](https://github.com/Kisakay/VeeMeeter/blob/main/branding/README.md).

## Meet Mimi

<div class="vm-gallery" markdown>
<figure><img src="../assets/brand/mascot/mimi-wave.svg" alt="Mimi waving"><figcaption>Waving · default</figcaption></figure>
<figure><img src="../assets/brand/mascot/mimi-happy.svg" alt="Mimi cheering"><figcaption>Happy · meters peaking</figcaption></figure>
<figure><img src="../assets/brand/mascot/mimi-sleepy.svg" alt="Mimi asleep"><figcaption>Sleepy · muted, empty</figcaption></figure>
<figure><img src="../assets/brand/mascot/mimi-surprised.svg" alt="Mimi surprised"><figcaption>Surprised · errors, 404</figcaption></figure>
<figure><img src="../assets/brand/mascot/mimi-thinking.svg" alt="Mimi thinking"><figcaption>Thinking · FAQ, tips</figcaption></figure>
<figure><img src="../assets/brand/mascot/mimi-wink.svg" alt="Mimi winking"><figcaption>Wink · playful asides</figcaption></figure>
<figure class="vm-on-dark"><img src="../assets/brand/mascot/mimi-wave-sticker.svg" alt="Mimi sticker"><figcaption>Sticker</figcaption></figure>
<figure class="vm-on-dark"><img src="../assets/brand/mascot/mimi-happy-sticker.svg" alt="Mimi sticker, cheering"><figcaption>Sticker</figcaption></figure>
</div>

**Mimi** (ミミ, "ears") is a tiny sound sprite who lives inside your PipeWire
graph. Her headphones are her meters: the bars on each ear cup bounce with
whatever she hears, and the mint knob on her headband is a fader she fiddles
with when she is nervous. Her sound-wave ahoge droops flat when a strip is
muted. Precise, a little perfectionist about gain staging, and she really,
*really* hates clipping.

## Logo

<div class="vm-gallery" markdown>
<figure class="vm-on-cream"><img src="../assets/brand/logo/mark.svg" alt="Mark"><figcaption><code>mark.svg</code> · app icon</figcaption></figure>
<figure class="vm-on-dark"><img src="../assets/brand/logo/mark-dark.svg" alt="Mark, dark"><figcaption><code>mark-dark.svg</code></figcaption></figure>
<figure class="vm-on-cream"><img src="../assets/brand/logo/mark-small.svg" alt="Mark, small sizes"><figcaption><code>mark-small.svg</code> · 16–48 px</figcaption></figure>
<figure class="vm-on-cream"><img src="../assets/brand/logo/mark-mono.svg" alt="Mark, monochrome"><figcaption><code>mark-mono.svg</code></figcaption></figure>
<figure class="vm-on-cream"><img src="../assets/brand/logo/mark-mono-inverse.svg" alt="Mark, monochrome inverse"><figcaption><code>mark-mono-inverse.svg</code></figcaption></figure>
</div>

<figure class="vm-on-cream" markdown>
![Horizontal logo](assets/brand/logo/logo-horizontal.svg){ width="560" }
</figure>

<figure markdown>
![Horizontal logo, dark](assets/brand/logo/logo-horizontal-dark.svg){ width="560" style="background:#1E1830;border-radius:18px;padding:12px" }
</figure>

- **Clear space**: at least the height of the wordmark's "e" on every side.
- **Minimum sizes**: mark 16 px (`mark-small`), full-detail mark 64 px,
  horizontal lockup 160 px wide.
- **Don't** add gradients, glows or blurred shadows, recolour outside the
  palette, stretch the wordmark or re-type it in a font.

## Palette

<div class="vm-swatches">
  <div class="vm-swatch"><span style="background:#FF8FB8"></span><p><b>sakura</b><br><code>#FF8FB8</code></p></div>
  <div class="vm-swatch"><span style="background:#B9A3F5"></span><p><b>lavender</b><br><code>#B9A3F5</code></p></div>
  <div class="vm-swatch"><span style="background:#7FE0C2"></span><p><b>mint</b><br><code>#7FE0C2</code></p></div>
  <div class="vm-swatch"><span style="background:#8CCBFF"></span><p><b>sky</b><br><code>#8CCBFF</code></p></div>
  <div class="vm-swatch"><span style="background:#FFE07A"></span><p><b>lemon</b><br><code>#FFE07A</code></p></div>
  <div class="vm-swatch"><span style="background:#FFF7EE"></span><p><b>cream</b><br><code>#FFF7EE</code></p></div>
  <div class="vm-swatch"><span style="background:#2B2140"></span><p><b>ink</b><br><code>#2B2140</code></p></div>
  <div class="vm-swatch"><span style="background:#1E1830"></span><p><b>night</b><br><code>#1E1830</code></p></div>
  <div class="vm-swatch"><span style="background:#C2185B"></span><p><b>sakura_deep</b><br><code>#C2185B</code></p></div>
  <div class="vm-swatch"><span style="background:#5B3FB8"></span><p><b>lavender_deep</b><br><code>#5B3FB8</code></p></div>
  <div class="vm-swatch"><span style="background:#1E7A5E"></span><p><b>mint_deep</b><br><code>#1E7A5E</code></p></div>
  <div class="vm-swatch"><span style="background:#FF5C7A"></span><p><b>cherry</b><br><code>#FF5C7A</code></p></div>
</div>

Pastels are **fills**; text on them is always **ink** (6.9–11.6 : 1). Coloured
text on light backgrounds uses the `_deep` shades (all ≥ 4.5 : 1). The app
theme (`[ui.dark]`, `[ui.light]`) and every checked ratio live in
[`palette.toml`](https://github.com/Kisakay/VeeMeeter/blob/main/branding/palette.toml).

## Typography

| Role | Font |
|---|---|
| Headings | <span style="font-family:Fredoka;font-weight:600;font-size:1.2em">Fredoka</span> |
| Body | Nunito (this text) |
| Code | `JetBrains Mono` |

## Decorations

<div class="vm-gallery" markdown>
<figure class="vm-on-cream"><img src="../assets/brand/decor/sparkle.svg" alt="Sparkle"></figure>
<figure class="vm-on-cream"><img src="../assets/brand/decor/star.svg" alt="Star"></figure>
<figure class="vm-on-cream"><img src="../assets/brand/decor/heart.svg" alt="Heart"></figure>
<figure class="vm-on-cream"><img src="../assets/brand/decor/note.svg" alt="Music note"></figure>
<figure class="vm-on-cream"><img src="../assets/brand/decor/eq-bars.svg" alt="Meter bars"></figure>
<figure class="vm-on-cream"><img src="../assets/brand/decor/mute-badge.svg" alt="Mute badge"></figure>
<figure class="vm-on-cream"><img src="../assets/brand/decor/pattern-light.svg" alt="Light pattern tile"></figure>
<figure class="vm-on-dark"><img src="../assets/brand/decor/pattern-dark.svg" alt="Dark pattern tile"></figure>
</div>

<img class="vm-divider" src="../assets/brand/decor/divider.svg" alt="Divider">

## Social preview

![VeeMeeter social preview](assets/brand/social/banner.png){ .vm-shot loading=lazy }

## Tone of voice

Playful and kawaii, but technically precise: the facts come first, the cute
is the seasoning. Kaomoji in moderation (one per section at most, never in
errors or CLI output) (｡•̀ᴗ-)✧
