# Lyrics V2

Lyrics V2 separates source parsing, playback time, presentation policy and drawing. The same
validated document and frame plan drive both renderers:

- `CairoLyricsView` is the correctness/reference backend. It shares the same binary-searched
  visible-line window as GL, so fallback rendering does not regress to scanning the whole song on
  every frame.
- `GlLyricsView` keeps compact Pango-rendered `R8` masks only for visible lines plus a two-line
  prefetch margin. Scrolling evicts masks outside that working set instead of retaining a texture
  for the whole song. Uniform locations, a unit quad and the per-frame command allocation are
  reused. The visible texture working set is represented by an allocation-free contiguous index
  window, and an upload list is allocated only when scrolling actually exposes an uncached line;
  ordinary frames do not rasterize text, allocate a texture-index set or upload vertex geometry.
  Two binary searches locate the conservative visible line range, so command generation visits
  only viewport-adjacent lines instead of walking the entire song on every frame. The range keeps
  the shared interlude displacement as safety margin, preventing a shifted boundary line from
  disappearing.
  It falls back to Cairo if GL setup or rendering fails. A runtime failure releases the hidden GL
  texture and layout caches before activating Cairo. The synchronized fallback remains dormant in
  the successful GL path instead of receiving an explicit redraw request on every player tick.
- `LyricsPresentation::Sidebar` uses the resolved application foreground on a solid surface.
  Album colour is ignored, and libadwaita dark/high-contrast changes re-resolve the foreground
  without restarting playback.
- `LyricsPresentation::Fullscreen` may tint inactive lyrics from the current album palette.

Line-timed sources stay line-timed. Karaoke highlighting is enabled only when the source contains
valid, non-zero-duration UTF-8 spans; no synthetic per-character timing is created.
An AMLL response is allowed to outrank NCM/QQ only after the V2 TTML parser confirms that it is
well-formed and contains timed lyric lines. Malformed or empty-shell TTML therefore falls back to
the usable provider result instead of replacing the current lyrics with a blank view.
The bounded asynchronous source cache also coalesces an in-flight request, so sidebar,
fullscreen and next-track preloading cannot fetch the same song from all providers more than once.
Source requests remain concurrent, but selection now finishes as soon as the declared priority makes
the answer definitive. A valid AMLL document does not wait for NCM/QQ, and NCM word timing does not
wait for QQ after AMLL is known to be unavailable. A lower-priority response still cannot bypass a
higher-priority request that is in flight. Switching tracks clears the old lyric view immediately,
including when the next track is pure music or its request fails.
Long source-authored gaps are represented once in the shared frame plan. Cairo and GL therefore
use the same opening/inter-line interlude timing, breathing dots and smooth 44 px layout push.
Explicit lyric/progress seeks clear manual scrolling and synchronize the viewport immediately.
Large position discontinuities without an explicit UI seek (including MPRIS seeks) are detected by
the shared viewport so they snap to the new focus instead of animating through unrelated lines.
The first lines may use a bounded negative automatic scroll so their centre reaches the same 34%
visual focus as the rest of the song; manual dragging is clamped at that designed leading position.
Cairo and GL share the corresponding viewport-to-content hit mapping, with a regression ensuring
that clicking the visually shifted first line still seeks to that line.

The visual hierarchy is also shared rather than tuned independently in each renderer. A normal
focus change uses a 520 ms ease-out handoff while the viewport follows at a deliberately slower
response than a seek. Sidebar lines scale from 0.99 to 1.0; fullscreen lines scale from 0.98 to
1.0. Past, future and active-unsung text have separate opacity policies, so the active karaoke
line no longer begins as an almost fully illuminated line. The active reveal uses a roughly 84 px
soft transition band instead of a hard-looking clip. Cairo applies the same line transform around
the main-text baseline that GL applies to its texture quad, scissor and reveal edge. All of these
effects are per-frame draw parameters: they do not invalidate Pango layout, allocate glyph
textures, or enlarge the bounded texture working set.
For one-character CJK source spans, the soft reveal clip is allowed to extend into the following
glyph while its timing edge remains source-authored. This produces a continuous travelling light
front instead of revealing one isolated glyph at a time. The next source range is also retained
during a sub-line timing gap, so the edge waits at the completed boundary rather than blinking off.

Lyric-line clicks have a separate viewport path from external seeks. A click preserves the current
viewport and animates toward the selected line; a distant selection starts at most 0.9 viewport
heights from the destination so it still communicates direction without flying through dozens of
lines. Progress-bar, MPRIS and track-change discontinuities retain their immediate synchronized
snap semantics.

## Rollout switch

The Settings dialog exposes **新版歌词渲染** as a restart-required rollback switch. It is on
by default. The backend choice is resolved once per process so sidebar and fullscreen cannot use
different generations after the setting changes. `LINN_LYRICS_V2` remains a developer/emergency
override and takes precedence over the saved switch:

| Value | Behaviour |
| --- | --- |
| unset + switch off | Existing lyric renderer (manual rollback) |
| unset + switch on | V2 GL renderer with automatic Cairo fallback (release default) |
| `legacy`, `off`, or unknown | Existing lyric renderer |
| `shadow` | Existing renderer plus V2 timeline comparison logs |
| `cairo` | V2 reference renderer |
| `gl` or `opengl` | V2 GL renderer with automatic Cairo fallback |

Turning the switch off and restarting, or launching once with `LINN_LYRICS_V2=legacy`, is the
immediate rollback and does not require migrating user data.

## Verification

Run the offline regression suite:

```sh
cargo test --all-targets -- --skip test_explore_apis
```

Run the live GL smoke test in a graphical session:

```sh
cargo test live_glarea_compiles_shaders_and_uploads_pango_texture -- --ignored --nocapture
```

Set `LINN_LYRICS_SMOKE_PNG=/tmp/linn-lyrics-v2-sidebar.png` to also save the fixed-size
light-sidebar fixture. The dump is composited onto its declared solid background instead of
exporting the GLArea's transparent framebuffer directly.

Set `LINN_LYRICS_FULLSCREEN_PNG=/tmp/linn-lyrics-v2-fullscreen.png` while running the fullscreen
reference test to save a headless Cairo fixture containing source-timed karaoke, translation,
album-tinted inactive text and the shared initial visual focus:

```sh
cargo test fullscreen_reference_fixture_renders_karaoke_and_translation -- --nocapture
```

The smoke test verifies shader compilation, real Pango and interlude-dot texture uploads, compact
texture width, shared interlude draw commands, non-background interlude pixels read back from the
actual GLArea framebuffer, and increased highlighted coverage between 25% and 75% of a real
source-authored span. It seeks into the middle of the document to prove bounded visible-line lookup
and GPU texture paging away from the first line. It also changes the sidebar foreground on the live
framebuffer and requires the new colour to render without rebuilding Pango layouts or line textures.
Later frames must reuse the command allocation, report a clean GL error state and release CPU mask
buffers after upload. The same live test finally injects a backend failure and requires the fallback
notification to switch a product-equivalent GTK stack to its synchronized Cairo child while the
shared lyric document remains available. GPU textures and the failed backend's Pango layouts must
also be released; this cleanup is exercised on both Wayland/EGL and X11/GLX. A separate headless
test checks that the GL soft
reveal uses the exact source-authored UTF-8 range and progress emitted by the shared timeline.
The headless Pango fixture matrix additionally lays out LRC, YRC, QRC and TTML in both sidebar and
fullscreen widths and requires every source-authored karaoke range to resolve to glyph geometry.
It rejects baseline-relative rectangles outside the layout texture; this guards both Cairo and GL
against silently clipping the karaoke layer away from the visible glyphs. Every line in the same
matrix is also rasterized to the compact 2× `A8` mask used for GL uploads and must contain real
glyph coverage. A Cairo image-surface regression independently requires highlighted pixel
coverage to grow from 25% to 75% source progress.

Run the repeatable release timeline baseline:

```sh
cargo test --release perf_baseline_one_million_frames -- --ignored --nocapture
```

Initial baseline on 2026-09-10 (Ryzen 7 8845HS): 1,000,000 frames across a synthetic 300-line,
1,800-span document completed in 0.033 seconds, or 33.2 ns/frame. Treat this as a local regression
reference, not a cross-machine requirement.

After adding renderer-independent interlude planning on 2026-09-12, the same release test measured
0.035 seconds (35.2 ns/frame), with sequential samples between 34.8 and 37.4 ns/frame. This small
absolute delta remains tracked rather than rounded away; renderer work in the same change removes
per-command uniform lookups, geometry uploads and command-vector allocation from the GL hot path.
After the seek-discontinuity and allocation-free texture-window changes on 2026-09-13, a fresh
release run measured 0.029 seconds (28.6 ns/frame). This is below both earlier samples; no timeline
performance increase was observed. After bounding GL command generation to the viewport-adjacent
line range with two binary searches, the same baseline measured 0.028 seconds (27.9 ns/frame).
The shared 10,000-line regression also limits geometry lookup to 32 probes and the returned range
to 10 lines for a 480 px viewport, preventing either Cairo drawing or GL command generation from
returning to a whole-song scan.

Run the headless GL mask working-set baseline:

```sh
cargo test --release gl_texture_memory_baseline -- --ignored --nocapture
```

On 2026-09-12, a 300-line fullscreen document at 2× scale would have required 47.31 MiB if every
compact line mask stayed resident (52.19 MiB at full width). The visible-window policy retained 12
lines and 1.89 MiB. The live smoke test also checks that the GPU texture keys exactly match the
current visible and prefetched line set, and that the per-frame command allocation is reused.
The same baseline repeated unchanged on 2026-09-13 after replacing the per-frame texture-index set
with the allocation-free window.
After adding line-focus transforms and the wider karaoke reveal on 2026-09-13, the timeline sample
remained 27.8 ns/frame and the GL mask baseline remained 12 lines / 1.89 MiB. The new animation
changes draw-command geometry and uniforms only; it did not expand the texture working set.

Before changing the default, manually cover sidebar and fullscreen on both light and dark themes,
line-only LRC, YRC/QRC, TTML, translations, long wrapped text, mixed CJK/Latin, RTL, pause/resume,
external seek, click seek, drag, wheel scrolling, rapid track changes and GL fallback. Validate both
opening and inter-line interludes, and both Wayland/EGL and X11/GLX where available.

Automated live framebuffer coverage completed on 2026-09-13 in a niri session for both the
application's XWayland/X11 path and a process-local native Wayland run. This proves context setup,
texture upload, interlude drawing and karaoke reveal on this host; it does not substitute for the
remaining manual song/theme checks or KDE, GNOME and Hyprland coverage.
