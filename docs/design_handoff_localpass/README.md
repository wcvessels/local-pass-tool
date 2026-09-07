# Handoff: LocalPass — local password generator widget

## Overview
LocalPass is a small, always-on-top desktop utility widget that generates cryptographically
random passwords **entirely on the user's machine** — nothing is stored, logged, or transmitted.
The UI is a compact frosted-glass floating window (356px wide) with a dark and a light theme,
an inline results list with masking and one-click copy, an auto-clearing clipboard timer, an
About/help pane, and a "roll up" collapsed state.

This bundle is the design reference for building that widget in a real app (Electron, Tauri,
a browser extension popup, a native shell with a webview, etc.).

## About the Design Files
The files in this bundle are **design references created in HTML** — a prototype of the intended
look and behavior, **not production code to ship directly**. Recreate this design in your target
environment using its established patterns and component libraries. If there is no environment
yet, pick the most appropriate stack for a small floating desktop utility (Electron/Tauri +
your preferred UI framework are natural fits) and implement it there.

Two files are included:
- **`LocalPass.standalone.html`** — a self-contained, dependency-free reimplementation
  (plain HTML/CSS/vanilla JS, no build step, no runtime). **Start here.** Open it in a browser
  to see the full behavior, and read its `<script>` — the generation logic, clipboard auto-clear,
  fit-to-width sizing, and all state transitions are written out plainly with comments. This is
  the source of truth for behavior.
- **`LocalPass Interactive (glass).dc.html`** — the original design-tool prototype. It relies on
  a proprietary template runtime (`support.js`, **not included and not needed**), so it will not
  run on its own. Use it only as a secondary cross-reference for exact markup/styles. Prefer the
  standalone file.

## Fidelity
**High-fidelity (hifi).** Colors, typography, spacing, radii, and interactions are final.
Recreate the UI pixel-accurately using your codebase's libraries. All exact values are in the
Design Tokens section and inline in the standalone file's CSS.

## Screens / Views
There is one window with four visual states (dark expanded, light expanded, info pane open,
rolled-up). The theme toggle swaps dark/light; the info pane and rolled-up bar are overlays/
replacements of the main card.

### 1. Main widget (expanded) — dark & light
- **Purpose**: Configure options, generate passwords, reveal/copy them.
- **Layout**: A single vertical flex column, `356px` wide, `border-radius: 12px`,
  `padding: 12px 16px 14px`, `gap: 11px` between sections. Frosted glass:
  `backdrop-filter: blur(18px) saturate(1.25)` over a semi-transparent gradient fill (see tokens).
  A soft decorative background (warm/cool/violet radial glows + two rotated light streaks) sits
  behind the window so the translucency reads — in a real desktop app the OS wallpaper provides
  this instead.
- **Components (top to bottom):**
  1. **Title bar** (`height: 24px`, flex row, `gap: 2px`):
     - `LOCALPASS` wordmark — 12px, weight 600, letter-spacing 2px, `flex: 1` (pushes icons right).
     - Icon buttons, left→right: **`?`** (About), **`✱`** (mask toggle), **`◉`** (pin-on-top),
       **theme toggle** (`◐` in dark / `◑` in light), **`✕`** (roll up). Each `24×24`,
       `border-radius: 5px`, `display:grid; place-items:center`, `font-size:11px` (theme icon 12px).
       Hover: bg `rgba(255,255,255,0.10)` (dark) / `rgba(31,34,38,0.10)` (light).
       `?`, `✱`, `◉` render in the **active** foreground color when their state is on, otherwise a
       **muted** color (see tokens). Theme toggle and `✕` use the idle icon color always.
  2. **Divider** — 1px top border (`rgba(255,255,255,0.09)` dark / `rgba(31,34,38,0.12)` light).
  3. **Length / slider / count row** (flex, `gap: 10px`, vertically centered):
     - `LENGTH` label (10px, 600, letter-spacing 1.2px, muted).
     - Numeric input, `34×26`, monospace 12px, centered, range **4–64**.
     - Custom **slider**, `flex: 1`, `height: 16px`, `cursor: ew-resize`. 2px track, filled
       portion + 10px square knob (`border-radius: 2px`). Maps position → length **4–64**.
     - `COUNT` label, then numeric input, range **1–99**.
  4. **Character-class segmented bar** (flex row, 1px border, `border-radius: 6px`, `overflow:hidden`):
     five cells, monospace 11.5px, `height: 30px`. Labels: `a–z`, `A–Z`, `0–9`, `!@#` (each
     `flex: 1`), and `NO 0O1l` (`flex: 1.3`). Each toggles independently. Selected vs unselected
     cells use different bg/fg (see tokens); cells are divided by 1px right borders.
  5. **Actions row** (flex, `gap: 8px`): **GENERATE** button (`flex: 1`, `height: 30px`, 11px/700,
     letter-spacing 1.4px, filled — light fill in dark theme, dark fill in light theme, drop
     shadow, `:active` nudges down 1px) and **CLEAR** button (`76px` wide, outlined). CLEAR is
     dimmed to `opacity: 0.38` with `cursor: default` when there are no results.
  6. **Results list** (only when passwords exist): preceded by a divider. A scrollable column
     (`max-height: 350px`, `overflow-y:auto`, thin themed scrollbar). Each row (`min-height: 34px`,
     flex, `gap: 10px`, 1px bottom border):
     - **Index** — 2-digit zero-padded (`01`, `02`…), monospace 10.5px, right-aligned, 18px wide.
     - **Password** — monospace, `flex: 1`, letter-spacing 0.6px, `word-break: break-all`. Font
       size is computed to **fit the row width** (see Interactions → fit-to-width). Clicking a
       password toggles reveal (when masking is on). Masked view shows `•` × password length.
     - **COPY button** — `height: 24px`, `padding: 0 8px`, monospace-ish 10px/600, letter-spacing
       1.2px. Label flips to `COPIED` for the copied row.
  7. **Clipboard toast** (only right after a copy): monospace 10.5px, amber
     (`oklch(0.78 0.13 75)` dark / `oklch(0.55 0.13 75)` light), text:
     `▮ COPIED 02 · CLIPBOARD RELEASES IN 30 S`, counting down each second.

### 2. Info / About pane
- **Purpose**: Explain what the controls do and set the clipboard-clear duration.
- **Layout**: A separate glass panel, `320px` wide, positioned to the **right** of the main card
  (`left: calc(100% + 14px); top: 0`), `z-index: 10`. Same glass treatment, `gap: 10px`.
  (In a real floating window this can be a right-hand flyout or a popover; keep it reachable —
  don't clip it.)
- **Components**: header row (`ABOUT LOCALPASS` label + `✕` close), divider, a privacy blurb,
  a legend list explaining `LENGTH · COUNT`, `a–z … !@#`, `NO 0O1l`, `✱`, `◉`, a divider, and a
  **Clipboard clear timer** row: label (`flex: 1`), the value (`30 S`, monospace, centered),
  then **`−`** and **`+`** step buttons **grouped on the right of the value**. Steps by **5**,
  clamped **5–60 s**; the `−`/`+` dim to `opacity: 0.35` at their limits.

### 3. Rolled-up bar (collapsed)
- **Purpose**: Minimized state — reclaim screen space while keeping the widget one click away.
- **Layout**: `356px` wide, `border-radius: 12px`, `padding: 8px 16px`, flex row. Sits at
  `opacity: 0.45`, rising to `1` on hover. Clicking anywhere expands it back.
- **Components**: `LOCALPASS` wordmark (`flex: 1`) + `CLICK TO EXPAND` hint (10px, 600,
  letter-spacing 1.2px, muted).

## Interactions & Behavior
- **Generate**: builds `count` passwords of `length` chars from the selected character classes.
  If no class is selected, it's a no-op. Resets reveal/copy state.
- **Password generation algorithm** (security-relevant — replicate exactly):
  1. Build the list of active character sets from the toggles.
  2. If **NO 0O1l** is on, strip look-alikes `I l 1 O 0 o 5 S 8 B` from every set.
  3. Drop any set that became empty.
  4. Seed the password with **one guaranteed character from each active set** (so every selected
     class is represented).
  5. Fill the remainder from the union of all active sets.
  6. **Fisher–Yates shuffle** the characters, then take the first `length`.
  - **All randomness uses a CSPRNG** (`crypto.getRandomValues` via a `rnd(n)` helper) — never
    `Math.random()`. In a Node/Electron main process use `crypto.randomInt`/`randomBytes`.
- **Length slider**: pointer-drag maps the x-position across the track to length **4–64**
  (`length = round(4 + p*60)`), live-updating the number field. Uses pointer capture via
  `pointermove`/`pointerup` on `window`.
- **Numeric inputs**: accept typing; commit-clamp on blur and on Enter (Enter blurs the field).
  Length clamps 4–64, count clamps 1–99.
- **Mask (`✱`)**: toggles masking of all results; clears any per-row reveals. When on, each
  password renders as bullets; **clicking a row reveals/hides just that row**.
- **Copy**: writes the password to the clipboard, sets that row's label to `COPIED`, shows the
  toast, and starts a **1-second-interval countdown** from the clipboard-clear value. At zero it
  **overwrites the clipboard with a space** (scrub) and clears the toast/label. Starting a new
  copy, generating, or clearing resets the timer. (Clipboard write is best-effort/try-caught.)
- **Clear**: empties results (no-op if already empty), stops the timer.
- **Theme toggle**: swaps dark ⇄ light. Icon shows `◐` (dark) / `◑` (light).
- **Pin (`◉`)**: visual toggle here; in a real app, wire to the OS "always on top" window flag.
- **Roll up (`✕`) / expand**: collapses to the rolled-up bar; clicking the bar expands.
- **Fit-to-width password sizing**: available text track ≈ **224px**; Cascadia Mono advance
  ≈ `0.62em` plus `0.6px` letter-spacing. Computed size `fit = (224 / len - 0.6) / 0.62`,
  capped at 12px. If `fit < 9px` it stops shrinking, **wraps** to multiple lines instead
  (`white-space: normal`, `padding: 6px 0`), and uses an 11px font.
- **Transitions**: icon-button bg/color `0.12s`; GENERATE bg `0.12s` + `translateY(1px)` on
  `:active`; rolled-up bar opacity `0.12s`. No other animation.

## State Management
State variables (all local/in-memory; nothing persisted):
- `theme` (`'dark' | 'light'`), `collapsed` (bool), `pinned` (bool), `masked` (bool)
- `length` (4–64), `count` (1–99)
- `lower, upper, digits, symbols, nolook` (bools — the five class toggles)
- `passwords` (string[]), `revealed` (map index→bool), `copiedIndex` (int|null), `copiedSecs` (int)
- `infoOpen` (bool), `clipSecs` (5–60, clipboard-clear duration)
- A single interval `timer` handle for the countdown (cleared on unmount, regenerate, clear, re-copy).

**No data fetching.** Everything is computed locally. This is a hard product requirement — the
privacy promise ("nothing stored, logged, or sent") means no network, no telemetry, no persistence.

### Configurable defaults
The prototype exposes three configuration defaults (surface as app settings if desired):
- `startTheme` — `'dark'` | `'light'` (default `dark`)
- `clipboardSeconds` — initial clear timer, 5–60 step 5 (default `30`)
- `maskPasswords` — start with results masked (default `false`)

## Design Tokens
**Fonts**
- UI: `"Segoe UI", system-ui, -apple-system, sans-serif`
- Monospace (all password text, numbers, timers): `"Cascadia Mono", Consolas, ui-monospace, monospace`

**Window fill (glass)**
- Dark card: `linear-gradient(180deg, rgba(40,45,52,0.52), rgba(17,20,24,0.62))`, border
  `rgba(255,255,255,0.16)`, shadow `0 16px 40px rgba(0,0,0,0.45)`
- Light card: `linear-gradient(180deg, rgba(255,255,255,0.45), rgba(236,240,244,0.55))`, border
  `rgba(255,255,255,0.65)`, shadow `0 16px 40px rgba(20,25,35,0.35)`
- Both: `backdrop-filter: blur(18px) saturate(1.25)` (include `-webkit-` prefix)
- Info pane fills are slightly denser: dark `0.55/0.65`, light `0.48/0.58` gradient stops.

**Text colors**
- Dark: primary `#f2f3f5`, active `#f2f3f5`, idle-icon `#b2b8be`, muted `#6a7078`,
  secondary body `#e8ebee`, row index `#8a9199`.
- Light: primary `#24262a` / `#1f2226`, idle-icon `#4f565e`, muted `#9aa0a6`, row index `#6a7078`.

**Buttons**
- GENERATE dark: bg `#eef1f4` → hover `#ffffff`, text `#16181c`.
- GENERATE light: bg `#23262b` → hover `#101215`, text `#f7f8f9`.
- Copy amber toast: `oklch(0.78 0.13 75)` (dark) / `oklch(0.55 0.13 75)` (light).

**Segmented cells**
- Dark on/off bg: `rgba(255,255,255,0.16)` / `rgba(255,255,255,0.03)`; fg `#f2f3f5` / `#79818a`.
- Light on/off bg: `rgba(31,34,38,0.12)` / `rgba(255,255,255,0.40)`; fg `#1f2226` / `#8a9098`.

**Geometry**
- Card width `356px`, info pane `320px`. Radii: card/pane/rolled `12px`; buttons/inputs `4px`;
  icon buttons/segbar `5–6px`; slider knob `2px`.
- Card padding `12px 16px 14px`; section gap `11px`; control-row gap `10px`.

**Decorative backdrop** (prototype only — real app uses the OS wallpaper):
page bg `linear-gradient(140deg, #46536b, #6d6a75 45%, #9b8677)` plus soft radial glows and two
rotated white streaks. Reproduced in the standalone file's `.bg-*` rules.

## Assets
No image or icon-file assets. All glyphs are Unicode characters rendered as text:
`?`, `✱` (U+2731), `◉` (U+25C9), `◐` (U+25D0), `◑` (U+25D1), `✕` (U+2715), `−` (U+2212),
`•` (U+2022), `▮` (U+25AE). Swap for your icon set if you prefer, but the current look uses the
plain glyphs. Fonts are system fonts (Segoe UI, Cascadia Mono / Consolas) — no web-font files.

## Files
- `LocalPass.standalone.html` — **primary reference.** Runnable, dependency-free reimplementation;
  read its `<script>` for exact behavior.
- `LocalPass Interactive (glass).dc.html` — original design-tool prototype (needs an absent runtime;
  markup/style cross-reference only).
- `README.md` — this document.
