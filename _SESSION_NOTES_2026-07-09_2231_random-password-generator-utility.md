# Session Notes
**Topic:** random-password-generator-utility
**Started:** 2026-07-09 22:31
**Session file:** _SESSION_NOTES_2026-07-09_2231_random-password-generator-utility.md
**Continuation:** Current Codex task; no handoff required.
---

## [2026-07-09 22:31] -- LocalPass build and third visual direction
- **Goal:** Build an unobtrusive Windows password-reset utility that runs locally as a double-click executable with no CLI needed for normal use.
- **Key points:** Generate 1-50 unique passwords at length 8-64. Default length 20 with lowercase, uppercase, numbers, safe symbols, and ambiguous characters excluded. Use cryptographic randomness and guarantee every enabled group occurs.
- **Key points:** No network calls, storage, logging, analytics, Node runtime, or external service. Generated values stay in memory/UI and are removed on clear, regeneration, and close.
- **Key points:** Clipboard copying is explicit. Windows Clipboard History/cloud exclusion marker is set before secret text. Clipboard ownership uses the Windows sequence number and clears after 30 seconds only if still owned.
- **Decisions:** Native dependency-free WPF on the built-in .NET Framework compiler. Portable output is dist/LocalPass.exe.
- **Decisions:** Managed strings and OS clipboard buffers cannot provide forensic zeroization; practical cleanup and low-risk trusted-workstation use are the stated boundary.
- **Decisions:** The initial WinForms UI was rejected as ugly, oversized, and barely translucent. It was replaced with a smaller WPF shell and per-row Copy actions.
- **User objection:** The second WPF pass is still rejected: green palette, flat controls, oversized Generate/results layout, and insufficient glass character.
- **Current direction:** Remove green. Provide session-only light and dark frosted themes: white/light-grey frost and gunmetal/dark-grey frost.
- **Current direction:** Replace large flat controls with compact three-dimensional glass-bubble controls. Move Generate into a small action row and remove the separate results heading/divider.
- **Current direction:** Roll the settings/results body up when inactive and pointer-outside; unroll on hover or activation. Mask passwords before rolling up.
- **Current direction:** Interpret the user's “90% transparency when active” as 90% opacity for readability; literal 10% opacity remains an open correction if the user meant it exactly.
- **Validation:** build.ps1 passes the embedded generator matrix/self-test and produced a 42 KB dist/LocalPass.exe in the latest completed build.
- **Validation:** Static WPF compact and generated-state renders exist under artifacts/. Previous live Computer Use launch was stopped by the user with Escape because it appeared frozen; the user has approved a careful retry.
- **Security review:** RNG uses rejection sampling over the selected pool, avoiding modulo bias and producing a uniform distribution conditioned on enabled-group coverage.
- **Security review:** Prior clipboard sequence race and abandoned clear-on-contention paths were fixed. Close now hides and retries before exit.
- **Open risk:** Session-ending currently performs only one clipboard clear attempt and ignores failure.
- **Open risk:** Clipboard retry calls can sleep up to 120 ms on the WPF dispatcher; timer retry should use a non-blocking single attempt while direct Copy can retain bounded retry.
- **Files:** src/Core.cs; src/Ui.cs; src/MainWindow.cs; src/MainWindow.Actions.cs; build.ps1; README.md; dist/LocalPass.exe.
- **Open:** Implement neutral dual themes, bubble styling, roll-up behavior, compact action/results layout, security review fixes, new renders over mixed backdrops, build/self-test, then retry live UI inspection without long blocking.

## [2026-07-09 23:01] -- Neutral frosted build verified
- **Completed:** Replaced the green palette with session-only gunmetal dark frost and white-grey light frost. Theme switching uses live WPF resources and no longer mutates sealed brushes.
- **Completed:** Window narrowed to 360 DIP. Generate, Clear, and per-row Copy are compact glass-bubble controls. Results have no redundant heading. Enabled policy toggles have visible state dots.
- **Completed:** Active opacity is 90 percent; inactive hover is 72 percent; inactive pointer-out is 22 percent. Settings/results roll into the header after 450 ms and unroll on hover or activation. Secrets mask before blur/roll-up.
- **Completed:** Clipboard timer retries are single-attempt and nonblocking. Direct copy/clear/normal-close retain bounded retries. Session shutdown uses bounded best effort because Windows owns final clipboard disposal.
- **Runtime fix:** Careful live testing found an immediate Generate-then-Clear focus race. The queued focus now captures the button object and checks visibility instead of indexing a cleared row.
- **Runtime fix:** Live clipboard probing found that Windows publishes the final clipboard sequence after CloseClipboard. Ownership now records the post-close sequence and validates sequence, owner HWND, and exclusion format before every clear.
- **Validation:** Embedded generator self-test passes. Rapid Generate/Clear was exercised 20 times in the live EXE without crash. Clear status was confirmed without capturing generated plaintext. Close removed the process.
- **Validation:** Final security review PASS. Final supported-Windows-10 UI review GO. Source scan found no network or storage APIs.
- **Validation:** Explicit Copy produced Unicode text plus the history/cloud exclusion marker at sequence 312. Clear advanced to 313 and removed both formats without reading the password content.
- **Artifact:** dist/LocalPass.exe, 51,200 bytes, SHA-256 6F83B71EA273F0169AD7E38A1517BC51260578F8D4426122E68F5947F2791F52. The local build is unsigned.
- **Platform note:** Windows 10 uses a supported translucent frosted-tint fallback. True supported system acrylic is a newer Windows 11 capability; undocumented composition hooks and desktop capture were rejected.
- **Continuation:** No fresh Codex task is required. Reopen this session note only if later polish or packaging work continues.
