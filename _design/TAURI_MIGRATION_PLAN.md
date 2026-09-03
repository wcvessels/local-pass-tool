# LocalPass Tauri 2 migration plan

This plan follows `_design/TAURI_ARCHITECTURE.md`. The architecture gate returned GO on 2026-07-13; Phase 1 may begin.

Resolved decision (2026-07-13): macOS best-effort `changeCount` clearing is available only behind an explicit session toggle that starts OFF every launch. The warning and narrower platform promise are required parity behavior. Scaffolding may start after the updated documents pass adversarial review.

## Implementation checkpoint - 2026-07-13

Landed beside the preserved WPF implementation:

- Minimal Tauri 2 crate, lockfiles, vanilla TypeScript/HTML/CSS UI, generated narrow command permissions, strict local CSP, and Rust navigation/new-window/download denial.
- Exact Rust generator, serialized clipboard actor, Windows dedicated-owner adapter, macOS default-OFF best-effort adapter, X11 source-owner adapter, Rust window/close/single-instance service, and platform bundle configuration.
- Truthful clipboard status sequencing, authoritative countdown polling, synchronous UI redaction, fail-closed browser hardening, and the exact macOS opt-in warning.

Local evidence: WPF build/self-test still passes; TypeScript and production Vite build pass; boundary verifier passes; Rust tests pass; locked offline Cargo check passes; Windows release no-bundle build and native launch pass.

Parity remains **NO-GO**. Native macOS/Linux CI and runtime smoke have not run; Wayland remains typed `Unsupported`; native OS High Contrast adapters, packaged-artifact tests, deny-outbound runtime tests, signing/notarization, and clean-VM evidence remain open. WPF and `README.md` therefore stay unchanged.


## Preservation rule

Keep these unchanged and runnable through feature parity:

- `src/Core.cs`
- `src/MainWindow.cs`
- `src/MainWindow.Actions.cs`
- `src/Ui.cs`
- `build.ps1`
- `dist/LocalPass.exe`
- `README.md` until a reviewed parity/release gate updates its WPF-only wording

The `codex/wpf-checkpoint` versions of these files are the behavioral/visual baseline. New Tauri files live beside the WPF implementation. Do not rename the WPF `src` directory during migration. Existing `_SESSION_NOTES_*.md` files stay untracked.

## Phase 0 — architecture gate

Deliverables:

- Architecture and trust boundary.
- Explicit reference precedence and known handoff security defects.
- WPF parity inventory.
- Platform clipboard decisions and Wayland release gate.
- Narrow command/capability list.

Exit proof:

- No Tauri scaffold, package install, or WPF edit occurred before the documents were written.
- Security review confirms no generic clipboard, filesystem, shell, HTTP, updater, opener, process, or persistence capability.

## Phase 1 — toolchain and inert scaffold

Work:

1. Install a supported Rust stable toolchain; record versions. Current machine has Node/npm but no Cargo/Rust.
2. Add a minimal vanilla TypeScript/Vite frontend and Tauri 2 Rust crate manually. No React and no template demo code.
3. Add local-only Tauri configuration, exact local capabilities, no plugin permissions, and static `create: false` window config.
4. Generate app-command permissions and separately register the same handlers at runtime. Compile every command, permission, handler, and `tauri_build` setup on every target; gate only adapter internals. Implement the architecture's exact `main`/`about` matrix and caller-label validation.
5. Register single-instance first, then create the hardened local webviews in Rust with production CSP plus navigation/new-window/download denial hooks already installed.
6. Add unsigned Windows, macOS, and Linux compile/test runners now and make all three minimal native compiles green before phase exit.
7. Put native crates in Cargo target-specific dependency sections. Commit `package-lock.json` and `Cargo.lock`; extend `.gitignore` for generated Node, Vite, Rust, Tauri, bundle, and test output only.

Exit proof:

- `npm run build` and `cargo check` succeed on the host; minimal Tauri compiles are green on native Windows, macOS, and Linux runners.
- Built frontend contains no remote URL.
- Config names only the two local capability identifiers. Capability files target exact labels, list only generated LocalPass allow-permissions, and validate cleanly against the generated desktop schema.
- Dependency tree contains no Tauri clipboard, HTTP, updater, shell, filesystem, opener, store, or logging plugin.
- Fixed loopback dev server uses port 1420, strict port, no HMR, and a separate loopback-only `devCsp`; production CSP remains unchanged.
- Window config disables persistence/autofill helpers, link preview, drag/drop, extensions, and devtools as specified by the architecture.

## Phase 2 — Rust generator vertical slice

Work:

1. Port the exact `Core.cs` groups, bounds, rejection sampler, group seeding, and Fisher-Yates shuffle.
2. Add validated serde DTOs and `generate_passwords`.
3. Store batches under a monotonic opaque ID with zeroizing Rust-owned strings.
4. Return the batch to a temporary plain frontend view; no clipboard yet.

Exit proof:

- Rust tests reproduce and strengthen the WPF self-test: all 15 masks, boundary lengths, count 99, invalid inputs, ambiguous removal, required groups, deterministic fake-RNG rejection/shuffle cases, and entropy failure.
- Search proves password generation does not exist in TypeScript.
- WPF self-test still passes.

## Phase 3 — clipboard actor and native adapters

Work order:

1. Define FIFO actor messages and `ClipboardLease` state, including captured clear policy, irreversible macOS `BestEffortArmed -> BestEffortDisarmed`, adapter-owned zeroizing payload, and request-loop lifetime. Add stale-generation cancellation and 250 ms busy retry.
2. Port the Windows transaction, including marker-before-text, a dedicated actor-owned HWND with message pumping, owner/sequence validation, double-checked owned clear, and WebView-independent retry lifetime.
   Use a lazy private message-only owner HWND on the clipboard actor thread; pump its messages and retain it through WebView destruction until release is terminal.
3. Add macOS AppKit copy adapter: prepare current-host-only contents and all cooperative markers before text publication, then capture `changeCount`. Default-OFF leases never mutate the pasteboard. Newly opted-in leases use best-effort check-then-clear with the acknowledged residual clobber race.
4. Add X11 dedicated selection-owner adapter. Never advertise, serve, or request `SAVE_TARGETS`.
5. Spike GNOME and KDE Wayland source-identity cleanup through the existing GTK/Tauri event loop, including seat and valid user-event serial access. No direct-protocol fallback unless that integration is first proven.
6. Expose only `copy_password`, `clear_sensitive_state`, `clipboard_status`, validated `set_clipboard_timeout`, and `set_macos_best_effort_clear`. Keep every command registered on every target; non-macOS policy calls return typed `Unsupported`.
7. Wire timeout, policy-eligible active-timeout updates, Clear, regeneration, close, and session-ending paths. Non-mutating/disarmed macOS leases stay deadline-free.

Exit proof:

- No frontend clipboard API and no Tauri clipboard plugin/capability.
- Browser-native COPY/CUT, selection copy, context-menu copy, drag, `execCommand("copy")`, and `navigator.clipboard` fail on all three webview engines.
- Copy uses batch ID and row index, never frontend-supplied plaintext. Rust validates timeout `5..60`, multiple of five, and updates an active lease from original copy time.
- Windows/X11/Wayland tests prove a newer clipboard value survives. macOS tests prove OFF never mutates or returns a deadline; ON arms only later copies; OFF->ON and timeout changes never rearm an old/disarmed lease; a mismatched `changeCount` prevents clear.
- Disable-versus-deadline tests prove FIFO ordering and acknowledgment only after disarm. Any earlier in-flight release finishes first and returns its actual outcome. Restart resets OFF. The unavoidable check/clear race is documented, never described as safe.
- Old timer cannot clear a new copy.
- Pre-commit failure retains the prior lease; destructive failure invalidates it; commit-uncertain enters quarantine and cleanup.
- Busy Windows clear retries; normal close hides until released; session ending performs bounded best effort.
- Windows owner-lifecycle tests prove the private HWND is lazy, stable across a lease, message-pumped on the actor, and destroyed only with the adapter.
- Run native unsigned clipboard smoke jobs during this phase, not after packaging.
- Wayland ships only if owner-source cleanup passes on GNOME and KDE. Failure blocks supported Wayland release and cross-platform parity; unsafe unconditional clear is not an option.

## Phase 4 — Rust window and single-instance service

Work:

1. Implement a 400 px expanded main layout, a shadow-free 40 px collapsed layout with separate guarded close, and a local shadow-free 320 px About popup.
2. Validate frontend-measured height, clamp both windows to monitor work area, and place About with a 14 px right, left, below, then above preference before allowing overlap.
3. Implement caller-owned main/About native drag, topmost, guarded close/destroy, app exit handling, blur/deactivation, redaction acknowledgment, and session-ending cleanup.
4. Implement 90% dark/light expanded surfaces with shell states of 1.0 focused, 0.95 inactive-hovered, and 0.85 inactive-idle. Add an About control for session-only collapsed opacity from 0.25 to 0.75 in 0.05 steps, defaulting to 0.50; keep hover localized and never expand on hover. Whole-app focus loss collapses after one event-loop handoff deferral; only collapsed-bar click or Enter/Space expands.
5. Preserve single-instance behavior: second launch exits silently without changing the existing window.
6. Add adapter-owned High Contrast/forced-colors behavior and platform blur/vibrancy only with readable fallback. Enable macOS private API explicitly and keep Mac App Store out of scope.

Exit proof:

- No broad `core:window:*` permission.
- Window commands accept no arbitrary label: main-only commands affect `main`; `close_about` affects only the calling `about` popup; `start_window_drag` accepts only its calling `main` or `about` window.
- Close cannot bypass clipboard cleanup during normal operation.
- Native close cannot recurse; unresponsive webview is hidden then destroyed while clipboard cleanup continues headlessly.
- DPI/scaling, taskbar/dock behavior, multi-monitor clamping, About-at-screen-edge, and High Contrast pass on all three OS families.

## Phase 5 — production vanilla UI

Work:

1. Rebuild the WPF checkpoint in semantic HTML, TypeScript, and CSS. Use the standalone handoff for geometry/tokens, not security logic.
2. Use native `<button>`, digit-only text inputs with `inputmode="numeric"`, and `<input type="range">` controls with labels and visible focus. Keep custom styling but preserve keyboard semantics.
3. Implement defaults: length 20, count 3, all groups on, ambiguous exclusion on, pin on, dark theme, clipboard 30 s.
4. Implement results with fixed one-line 11px typography and character ellipsis, mask/reveal, About pane, timer controls, status/warning states, Ctrl+G, and Ctrl+L. Preserve each complete password for COPY. On macOS only, add a best-effort auto-clear toggle in About. Keep it OFF by default and session-only; render and associate the exact warning from the architecture. OFF disables timed-clear controls, has no release deadline, and never shows a false countdown or clipboard-cleared claim.
5. Mask all rows on blur before any roll-up transition. Do not render secrets into attributes, logs, accessibility descriptions, browser storage, or autofill-enabled fields. Results are non-selectable/non-draggable and block copy/cut/context-menu paths.
6. Add synchronous UI redaction before Clear, regenerate, and header close. Rust-initiated native close uses nonce acknowledgment; command failure leaves old UI redacted.
7. Reproduce numeric behavior exactly: reject non-digit typing/paste, Enter blurs, Up/Down steps one, invalid length falls back to 20, invalid count to 1, then clamp.
8. Respect High Contrast, reduced motion, dark/light contrast, system fonts, zoom, and keyboard-only use. High Contrast forces system colors/opaque defaults, hides theme, and disables custom glass/shadow/fades/auto-collapse.

Exit proof:

- Visual captures exist for dark, light, About, rolled, long/ellipsized results, 99-result scroll, High Contrast, and error/pending states.
- Keyboard-only path covers every control; accessible names/states match WPF intent.
- Theme/window choices disappear on restart.
- macOS best-effort consent also disappears on restart. Enabling affects future copies only. During disable, the toggle/status remains pending until actor acknowledgment; afterward the UI shows OFF and the returned prior-release outcome. OFF->ON never rearms the old copy. The warning remains visible and keyboard/screen-reader accessible.
- No class selected remains a no-op, matching WPF.
- Inactive results are always masked.
- Regeneration redacts/removes old DOM immediately, starts prior clipboard release without blocking, and can show the new batch with a release-pending status.

## Phase 6 — security and offline gate

Work:

1. Audit the navigation, new-window, and download denial hooks installed at builder time in Phase 1.
2. Apply strict CSP and local asset rules from the architecture.
3. Audit generated capabilities, npm packages, Cargo crates/features, bundle contents, and release configuration.
4. Add negative tests for external fetch, websocket, navigation, popup, download, remote image, unauthorized IPC, path access, shell launch, and clipboard APIs.
5. Run the packaged app under deny-all outbound firewall/network namespace.
6. Verify no logs contain secrets and release builds contain no devtools/updater endpoint.

Exit proof:

- All negative tests fail closed.
- App generates, copies, times out, clears, and closes while offline.
- Dependency and capability audit is attached to the release checklist.

## Phase 7 — native build matrix and packaging

Work:

1. Windows x64 NSIS with offline WebView2 installer using `--bundles nsis`.
2. macOS Apple Silicon and Intel signed/notarized DMGs using `--bundles dmg`.
3. Linux x64 AppImage and `.deb` from a fixed WebKitGTK baseline using `--bundles appimage,deb`.
4. Native-runner unit, build, and smoke jobs. Keep signing/notarization secrets only in CI secret storage.
5. Run clipboard-manager matrix: Windows history/cloud sync, macOS Universal Clipboard plus a third-party manager, X11 with/without manager, GNOME Wayland, KDE Wayland.

Exit proof:

- Clean VM install/launch/uninstall evidence for each artifact.
- Runtime outbound-denial check passes on every OS.
- Clipboard and single-instance smoke tests pass on native OS runners.
- Linux unsupported/degraded compositor behavior, if any, is explicit in README and release notes.
- `bundle.createUpdaterArtifacts` is false and default `all` bundle targets are never used.

## Phase 8 — parity decision

Required review packet:

- WPF-versus-Tauri behavior checklist with pass/fail evidence.
- Generator and clipboard test results.
- Cross-platform UI captures.
- Capability/dependency/network audit.
- Known platform limitations and exact release README wording affirmatively covering every macOS policy fact required by the architecture.
- Artifact hashes and native build provenance.

Decision:

- If the macOS default-OFF toggle, exact risk warning, or lease-scoping behavior is missing: no parity declaration.
- If any security, clipboard ownership, inactive masking, accessibility, or offline gate fails: keep WPF as the supported Windows release and continue migration.
- If every required gate passes: declare Tauri feature parity. Preserve WPF in repository history and optionally move live WPF files to a clearly named `legacy-wpf/` folder in a separate reviewed commit.
- Never delete the WPF reference in the same commit that first claims parity.

## Acceptance checklist

### Generator

- [ ] Exact groups and ambiguous characters.
- [ ] Count 1–99; length 4–64.
- [ ] Every selected group represented.
- [ ] Unbiased CSPRNG selection and Fisher-Yates shuffle.
- [ ] No entropy fallback, persistence, logging, or network.

### Clipboard

- [ ] Explicit per-row COPY only.
- [ ] Timeout 5–60 s in 5 s steps; default 30 s.
- [ ] Regenerate, Clear, timeout, normal close, and session end request policy-governed release; macOS OFF/disarmed does not mutate.
- [ ] Windows and Linux source-identity adapters never erase another application's newer value.
- [ ] macOS best-effort clear starts OFF every launch, is never persisted, and OFF never mutates pasteboard content.
- [ ] Enabling affects future copies only; disable acknowledgment irreversibly disarms the lease; OFF->ON and timeout changes cannot rearm it; `changeCount` mismatch prevents clear.
- [ ] About keeps the architecture's exact two-sided warning visible and accessible. Release README affirmatively states default OFF/session-only, all OFF non-clearing triggers, future-copy-only scope, residual clobber race, and reset on exit; mere omission of atomic-safety language fails.
- [ ] Windows built-in history/cloud exclusion preserved.
- [ ] macOS/Linux history/sync claims are platform-specific and honest.

### UI and window

- [ ] Defaults, masking, reveal, rows, one-line ellipsis, status, theme, About, pin, drag, and explicit collapsed expansion.
- [ ] Hover never expands; outside focus collapses after one handoff deferral; About focus suppresses collapse; collapsed close uses guarded redaction.
- [ ] Ctrl+G/Ctrl+L and numeric keyboard behavior.
- [ ] Inactive results always masked.
- [ ] High Contrast disables glass/auto-collapse effects.
- [ ] Second launch exits silently and leaves the existing window unchanged.
- [ ] Session-only state.

### Boundary and release

- [ ] Local assets and strict CSP.
- [ ] No remote content, telemetry, network plugin, updater, broad filesystem, shell, opener, process, or generic clipboard permission.
- [ ] Navigation, popup, and downloads denied in Rust.
- [ ] Windows, macOS, X11, GNOME Wayland, and KDE Wayland native evidence.
- [ ] WPF remains until reviewed parity declaration.
