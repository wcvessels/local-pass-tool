# Session Notes
**Topic:** random-password-generator-utility
**Started:** 2026-07-12 17:44
**Session file:** _SESSION_NOTES_2026-07-12_1744_random-password-generator-utility.md
**Continuation:** LocalPass WPF design rework and cross-platform stack decision
---

## [2026-07-12 17:44] -- WPF design baseline and Tauri migration handoff
- **Goal:** Preserve the working LocalPass WPF redesign as a checkpoint, then continue cross-platform development in a fresh session.
- **Key points:** The user wants Windows, macOS, and Linux support. The supplied Claude glass design is the visual source of truth under `_design/design_handoff_localpass`; current WPF remains the behavioral reference until parity.
- **Work completed:** Rebuilt the WPF UI around the dark/light glass design; widened the card to 400 px and window to 420 px; fixed password clipping; made X close the app; made collapse an automatic inactive/outside effect; added hover/focus expansion, segmented controls, About flyout, mask/reveal, configurable clipboard clearing, and single-instance behavior.
- **Security/core behavior:** Password length is 4-64 and count is 1-99. Generation guarantees at least one character from every enabled set, fills from the union, shuffles with Fisher-Yates, and uses cryptographic randomness. Lookalikes exclude `Il1O0o5S8B`.
- **Decisions:** Use Tauri 2 for the cross-platform migration. Use vanilla TypeScript/HTML/CSS for UI and Rust for password generation, clipboard lifecycle, single-instance behavior, window management, and platform adapters. Do not add React.
- **Constraints:** Bundle local assets only. No remote content, telemetry, network access, or broad filesystem/shell permissions. Use strict Tauri capabilities and CSP. Preserve the WPF implementation until feature parity.
- **Platform note:** Clipboard ownership and hardening need explicit Windows, macOS, X11, and Wayland adapters; Windows clipboard-history exclusion cannot be assumed portable.
- **Validation:** `build.ps1` passed its self-test and rebuilt `dist/LocalPass.exe` at 52 KB. `git diff --check` passed with only expected LF-to-CRLF warnings. Earlier UI behavior checks passed for 450 ms inactive roll-up, body expansion, X close, dark/light/generated/About/rolled rendering, and duplicate-instance rejection.
- **Checkpoint:** Branch `codex/wpf-checkpoint`; commit `e048926` (`Checkpoint WPF glass UI before Tauri migration`). The commit contains 15 app/design files. Session notes remain untracked.
- **Browser/tooling limits:** The Codex in-app browser blocked direct `file://` preview. Transparent borderless-window capture also failed, so UI verification used a safe offscreen render harness.
- **Files:** `README.md`, `src/Core.cs`, `src/MainWindow.Actions.cs`, `src/MainWindow.cs`, `src/Ui.cs`, `dist/LocalPass.exe`, and `_design/`.
- **Open:** Create the Tauri 2 architecture/migration plan; scaffold only after the plan; implement per-platform clipboard/window behavior; establish feature-parity and packaging tests for all three operating systems.

---
## Session Summary
**Ended:** 2026-07-12 17:46
**Duration:** 2026-07-12 17:44 to 2026-07-12 17:46

### What was accomplished
- Captured the completed WPF glass-design rework and its security-sensitive behavior.
- Rebuilt and self-tested the Windows executable.
- Preserved the supplied Claude design handoff in Git as the future UI reference.
- Created a clean local checkpoint branch and commit before migration.
- Selected Tauri 2 plus vanilla web UI and a Rust system/security core for cross-platform work.

### Decisions made
1. Migrate with Tauri 2 because the HTML/CSS handoff can be used directly while keeping a small native shell.
2. Keep the UI framework-free unless complexity later proves a framework necessary.
3. Put generator, clipboard lifecycle, single-instance, window management, and platform adapters in Rust.
4. Keep WPF intact until the Tauri version reaches tested feature parity.
5. Keep the desktop app local-only and narrowly permissioned.

### Current state / handoff
The verified WPF baseline is committed on `codex/wpf-checkpoint` at `e048926`. No push was requested or performed. The next session should plan the Tauri architecture before scaffolding and should treat `_design/design_handoff_localpass`, `README.md`, and `src/Core.cs` as the primary starting references.

### Validation evidence
- `.\build.ps1` -> `LocalPass self-test passed.`
- Build output -> `dist\LocalPass.exe` at 52 KB.
- `git diff --check` -> no errors; line-ending warnings only.
- Staged-file audit -> 15 intended app/design files; no session notes.

### Recommendations
1. Define a small command/event boundary between the webview and Rust before writing UI code.
2. Separate clipboard implementations by platform and document capability differences, especially Wayland.
3. Add parity tests for generation, clear timing, masking, window roll-up, close behavior, and single-instance behavior.

### Concerns / risks
- Exact clipboard-clear and clipboard-history guarantees differ across platforms and desktop environments.
- Frameless/translucent window effects and inactive-window behavior require platform-specific verification.
- A browser-only visual match is insufficient; packaged builds must be checked on native DPI/scaling combinations.

### Files produced
- `_SESSION_NOTES_2026-07-12_1744_random-password-generator-utility.md`
- `_SESSION_NOTES_random-password-generator-utility.md`

### Unresolved
- Tauri directory/module layout and command API.
- macOS and Linux CI/package strategy.
- Exact per-platform clipboard guarantees and fallback copy.

### Continuation prompt
Migrate LocalPass from WPF to Tauri 2 for Windows, macOS, and Linux. Preserve current WPF implementation as reference until feature parity. Use vanilla TypeScript/HTML/CSS for UI and Rust for password generation, clipboard lifecycle, single-instance behavior, window management, and platform adapters. No React, remote content, telemetry, network access, or broad filesystem/shell permissions. Start by reading `_design/design_handoff_localpass`, current `README.md`, and security-sensitive behavior in `src/Core.cs`. Produce architecture and migration plan before scaffolding.
