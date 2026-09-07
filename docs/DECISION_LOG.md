# LocalPass decision log

This log records durable product, architecture, security, UI, and delivery choices. It is append-only: add a new entry to change a decision, mark the old entry superseded, and link both entries. Do not use it for transient test results, review findings, or task handoffs.

The initial entries normalize decisions made during the 2026-07-12 LocalPass UI and cross-platform session. Later implementation evidence is included only to show current disposition; unrelated September remediation decisions are out of scope.

## Status vocabulary

- **Accepted** — governing decision.
- **Provisional** — usable direction awaiting named evidence.
- **Blocked** — decision cannot advance until a named condition is met.
- **Superseded** — replaced by a later decision entry.
- **Partially superseded** — one requirement replaced by a later entry; the rest still governs.

## Index

| ID | Date | Status | Area | Decision |
|---|---|---|---|---|
| [LP-001](#lp-001--target-windows-macos-and-linux) | 2026-07-12 | Accepted | Product | Target Windows, macOS, and Linux |
| [LP-002](#lp-002--use-tauri-2-for-the-cross-platform-app) | 2026-07-12 | Accepted | Architecture | Use Tauri 2 for the cross-platform app |
| [LP-003](#lp-003--keep-the-frontend-framework-free) | 2026-07-12 | Accepted | Architecture | Use vanilla TypeScript, HTML, and CSS; no React |
| [LP-004](#lp-004--keep-privileged-behavior-in-rust) | 2026-07-12 | Accepted | Security | Keep privileged and security-sensitive behavior in Rust |
| [LP-005](#lp-005--keep-localpass-local-only-and-narrowly-permissioned) | 2026-07-12 | Accepted | Security | Keep LocalPass local-only and narrowly permissioned |
| [LP-006](#lp-006--separate-visual-and-behavioral-authority) | 2026-07-12 | Partially superseded | Delivery | Use the handoff for visuals and WPF for behavior until parity |
| [LP-007](#lp-007--close-with-x-and-collapse-on-inactivity) | 2026-07-12 | Accepted | Window/UI | X closes; inactivity drives collapse |
| [LP-008](#lp-008--prefer-content-fit-over-a-fixed-compact-width) | 2026-07-12 | Accepted | Window/UI | Expand or adapt width rather than clip password rows |
| [LP-009](#lp-009--preserve-the-verified-password-generation-contract) | 2026-07-12 | Accepted | Security | Preserve the verified generator contract |
| [LP-010](#lp-010--use-platform-specific-clipboard-policies) | 2026-07-12 | Accepted | Clipboard | Use explicit Windows, macOS, X11, and Wayland policies |
| [LP-011](#lp-011--checkpoint-and-plan-before-rewriting) | 2026-07-12 | Accepted | Delivery | Preserve a validated baseline and plan before scaffolding |
| [LP-012](#lp-012--route-security-reports-privately-once-the-repo-is-public) | 2026-09-07 | Blocked | Delivery | Route security reports through GitHub private vulnerability reporting once the repo is public |
| [LP-013](#lp-013--retire-the-wpf-implementation-from-the-working-tree) | 2026-09-07 | Accepted | Delivery | Retire the WPF implementation from the working tree; preserve it by tag |

## Decisions

### LP-001 — Target Windows, macOS, and Linux

- **Status:** Accepted
- **Area:** Product
- **Context:** WPF limited LocalPass to Windows. Cross-platform support became an explicit product requirement.
- **Decision:** Ship one LocalPass desktop product for Windows, macOS, and Linux.
- **Rationale:** Platform reach is a requirement, not an optional future enhancement.
- **Consequences:** Platform-specific security and window behavior must be stated and tested separately. A successful Windows build is not proof of macOS or Linux runtime parity.
- **Current disposition:** Tauri code and build configuration exist for all three targets. Native parity remains evidence-driven; unsupported behavior must stay explicit.
- **Evidence:** [Session handoff](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_SESSION_NOTES_2026-07-12_1744_random-password-generator-utility.md), [architecture goals](TAURI_ARCHITECTURE.md#goals)

### LP-002 — Use Tauri 2 for the cross-platform app

- **Status:** Accepted
- **Area:** Architecture
- **Context:** The supplied design is already HTML/CSS, while the application needs native window and clipboard integration.
- **Decision:** Migrate LocalPass to Tauri 2.
- **Rationale:** Tauri can reuse the design medium, keep the native shell small, and place security-sensitive work in Rust without bundling a Chromium runtime.
- **Alternatives considered:** Electron offered the easiest Chromium-consistent rendering but a larger runtime and distribution footprint. Avalonia kept .NET but required recreating the HTML/CSS design. A full-Rust UI such as Slint reduced language mixing but also required a visual rewrite and accepted a smaller UI ecosystem.
- **Consequences:** The app depends on each platform's system webview and needs native rendering checks. Tauri permissions and commands become part of the security boundary.
- **Current disposition:** Merged to `main` via PRs #1-#3 (2026-09-07).
- **Evidence:** [Session decision](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_SESSION_NOTES_2026-07-12_1744_random-password-generator-utility.md#decisions-made), [accepted architecture](TAURI_ARCHITECTURE.md)

### LP-003 — Keep the frontend framework-free

- **Status:** Accepted
- **Area:** Architecture
- **Context:** LocalPass has a small, fixed interaction surface and an existing HTML/CSS design handoff.
- **Decision:** Use vanilla TypeScript, HTML, and CSS. Do not add React or another UI framework unless a later decision demonstrates a concrete need.
- **Rationale:** This keeps dependencies, generated scaffolding, bundle size, and browser-side authority small.
- **Consequences:** UI state and lifecycle code remain explicit. A framework must not be added for convenience alone.
- **Current disposition:** Implemented in `frontend/` and `index.html`.
- **Evidence:** [Architecture header](TAURI_ARCHITECTURE.md), [architecture non-goals](TAURI_ARCHITECTURE.md#non-goals)

### LP-004 — Keep privileged behavior in Rust

- **Status:** Accepted
- **Area:** Security
- **Context:** Password generation, clipboard ownership, single-instance behavior, and native window operations need stronger control than generic browser APIs provide.
- **Decision:** Rust owns password generation, clipboard lifecycle, single-instance handling, window management, and platform adapters. The frontend gets only narrow task-specific commands.
- **Rationale:** Native authority stays behind a small, typed boundary. The webview cannot acquire generic clipboard, filesystem, shell, process, opener, updater, or network powers.
- **Consequences:** Command schemas and capabilities require review when changed. Password copy uses an opaque batch ID and row index rather than a generic plaintext write command.
- **Current disposition:** Implemented in `src-tauri/src/` with generated command permissions.
- **Evidence:** [Process architecture](TAURI_ARCHITECTURE.md#process-architecture), [command surface](TAURI_ARCHITECTURE.md#command-surface)

### LP-005 — Keep LocalPass local-only and narrowly permissioned

- **Status:** Accepted
- **Area:** Security
- **Context:** A password utility should minimize data egress and ambient authority.
- **Decision:** Bundle local assets only. Provide no runtime remote content, telemetry, network access, broad filesystem access, or shell access. Enforce a strict CSP and narrow Tauri capabilities.
- **Rationale:** Fewer egress paths and privileges reduce exposure of generated secrets and reduce supply-chain/runtime complexity.
- **Consequences:** Any feature requiring remote content, network egress, broad host access, or a wider Tauri capability needs an explicit superseding decision and security review.
- **Current disposition:** Reflected in the architecture, capabilities, frontend hardening, and boundary verifier. The 2026-07-13 architecture adds further non-goals; those are later decisions, not backdated here.
- **Evidence:** [Webview and network boundary](TAURI_ARCHITECTURE.md#webview-and-network-boundary), [migration security gate](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_design/TAURI_MIGRATION_PLAN.md#phase-6--security-and-offline-gate)

### LP-006 — Separate visual and behavioral authority

- **Status:** Partially superseded by [LP-013](#lp-013--retire-the-wpf-implementation-from-the-working-tree). The tree-preservation requirement is retired; the split between visual and behavioral authority still governs.
- **Area:** Delivery
- **Context:** The Claude HTML/CSS handoff describes the intended appearance, but its sample generation and clipboard code is not production security logic.
- **Decision:** Use `docs/design_handoff_localpass` as the visual reference. Preserve the WPF checkpoint as the behavioral and security reference until the Tauri app passes reviewed parity gates.
- **Rationale:** This retains the supplied visual direction without importing insecure prototype behavior or losing the verified Windows baseline.
- **Consequences:** When sources disagree, security/lifecycle behavior outranks prototype scripting. WPF source and build artifacts remain in the repository until an explicit parity decision retires them.
- **Current disposition:** Formalized by the architecture authority order. WPF was retired from the working tree by LP-013 and is preserved at tags `wpf-checkpoint` and `wpf-final`.
- **Evidence:** [Authority order](TAURI_ARCHITECTURE.md#authority-order), [glass handoff](<design_handoff_localpass/LocalPass Interactive (glass).dc.html>), [standalone behavior reference](design_handoff_localpass/LocalPass.standalone.html), checkpoint `e048926`

### LP-007 — Close with X and collapse on inactivity

- **Status:** Accepted
- **Area:** Window/UI
- **Context:** The earlier rework made close and collapse behavior ambiguous.
- **Decision:** The X control closes the application. Collapse is an automatic window-state effect driven by loss of activity or focus; X must never act as collapse.
- **Rationale:** Close must be predictable. The compact rail is passive window behavior, not a second meaning for the close control.
- **Consequences:** Native close must redact sensitive UI state and release clipboard state before shutdown. This entry defines the close-versus-collapse invariant, not the exact collapse timing or expansion gesture.
- **Current disposition:** The invariant remains governing. The accepted architecture later defines whole-app focus loss, deferred collapse, and explicit expansion behavior; hover alone does not expand.
- **Evidence:** [Session handoff](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_SESSION_NOTES_2026-07-12_1744_random-password-generator-utility.md), [window behavior](TAURI_ARCHITECTURE.md#window-behavior)

### LP-008 — Prefer content fit over a fixed compact width

- **Status:** Accepted
- **Area:** Window/UI
- **Context:** Password rows clipped at the earlier window width.
- **Decision:** Expand or adapt the application width when necessary to preserve usable, one-line password rows. Do not retain a narrower width at the cost of clipping.
- **Rationale:** The generated password and its copy action are the primary output. Their usability outranks an arbitrary compact width.
- **Consequences:** The WPF checkpoint used a 400 px card inside a 420 px window. Those numbers are a baseline, not a universal cross-platform constant; zoom, DPI, work area, and platform chrome may require adaptive dimensions.
- **Current disposition:** Implemented with platform/window sizing logic; native DPI and work-area behavior remain test obligations.
- **Evidence:** [Session handoff](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_SESSION_NOTES_2026-07-12_1744_random-password-generator-utility.md), [window behavior](TAURI_ARCHITECTURE.md#window-behavior)

### LP-009 — Preserve the verified password-generation contract

- **Status:** Accepted
- **Area:** Security
- **Context:** The prototype's generator is a visual demo and does not define production randomness behavior.
- **Decision:** Keep length 4–64 and count 1–99. Use cryptographic randomness, guarantee at least one character from every enabled set, fill from the enabled-set union, shuffle with Fisher–Yates, and exclude `Il1O0o5S8B` when lookalike filtering is active.
- **Rationale:** Migration must not weaken already verified generator behavior.
- **Consequences:** Prototype `Math.random` logic is never authoritative. Generator changes require statistical/contract tests and parity review.
- **Current disposition:** Ported to Rust. The WPF implementation is the historical reference at tag `wpf-checkpoint` (LP-013).
- **Evidence:** [Password generator architecture](TAURI_ARCHITECTURE.md#password-generator), [WPF generator](https://github.com/wcvessels/local-pass-tool/blob/wpf-checkpoint/src/Core.cs)

### LP-010 — Use platform-specific clipboard policies

- **Status:** Accepted
- **Area:** Clipboard
- **Context:** Clipboard ownership, history suppression, and clearing guarantees differ materially across Windows, macOS, X11, and Wayland.
- **Decision:** Implement and document separate platform adapters and promises. Do not project Windows clipboard guarantees onto other platforms.
- **Rationale:** A single generic clipboard abstraction would overstate safety or erase newer clipboard content on platforms without atomic ownership checks.
- **Consequences:** Each platform needs its own release semantics, race analysis, UI wording, and native runtime evidence. Unsupported behavior must fail honestly rather than simulate a guarantee.
- **Current disposition:** Exact semantics were deferred at the end of this session. The 2026-07-13 architecture later defined Windows, macOS, X11, and Wayland policies; those later choices should receive separate dated entries if this log is expanded beyond the requested session.
- **Evidence:** [Clipboard actor and adapters](TAURI_ARCHITECTURE.md#clipboard-actor-and-adapters), [migration clipboard phase](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_design/TAURI_MIGRATION_PLAN.md#phase-3--clipboard-actor-and-native-adapters)

### LP-011 — Checkpoint and plan before rewriting

- **Status:** Accepted
- **Area:** Delivery
- **Context:** The UI rework was usable enough to preserve, while the cross-platform migration changed the application architecture.
- **Decision:** Validate and checkpoint the WPF baseline, continue the migration in a fresh work session, and write the architecture and migration plan before scaffolding Tauri beside WPF.
- **Rationale:** This provides a recoverable reference and settles security boundaries before implementation spreads across UI and native code.
- **Consequences:** The checkpoint is provenance, not the active migration branch. Architecture gates precede implementation, and WPF is not deleted during migration.
- **Current disposition:** Completed. WPF checkpoint: tag `wpf-checkpoint` (`e048926`). Architecture gate later returned GO; migration proceeded on `codex/tauri-migration` and merged to `main`. WPF was retired from the tree by LP-013.
- **Evidence:** [Migration preservation rule](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_design/TAURI_MIGRATION_PLAN.md#preservation-rule), [session handoff](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_SESSION_NOTES_2026-07-12_1744_random-password-generator-utility.md)

### LP-012 — Route security reports privately once the repo is public

- **Status:** Blocked
- **Area:** Delivery
- **Context:** The README review on 2026-09-07 found that the Feedback section sends security concerns to the public issue tracker, and the repository has no `SECURITY.md` or private reporting path. GitHub private vulnerability reporting was attempted the same day and returned 404 because the repository is private; GitHub offers the feature only on public repositories.
- **Decision:** When the repository becomes public, enable GitHub private vulnerability reporting and change the README Feedback section to send security concerns to the Security tab's "Report a vulnerability" path. Bug reports and platform test results stay on issues.
- **Rationale:** With no release and a private repository, public issues expose nothing. Once the repository and a release are public, a public issue describing a clipboard or generator defect is a working exploit note for the window before the fix. The feature costs one API call and signals a maintained security posture to the IAM and secrets-management readers the README targets.
- **Consequences:** Going public has a two-step checklist, not one. A `SECURITY.md` is optional; the GitHub button is sufficient.
- **Blocked on:** Repository visibility change to public.
- **Unblock command:** `gh api -X PUT repos/wcvessels/local-pass-tool/private-vulnerability-reporting`
- **Current disposition:** Repository is private as of 2026-09-07. README Feedback section unchanged pending the visibility change.
- **Evidence:** [README Feedback section](../README.md#feedback), [GitHub private vulnerability reporting docs](https://docs.github.com/en/code-security/security-advisories/working-with-repository-security-advisories/configuring-private-vulnerability-reporting-for-a-repository)

### LP-013 — Retire the WPF implementation from the working tree

- **Status:** Accepted
- **Area:** Delivery
- **Context:** The Tauri app is built and manually verified on Windows and merged to `main`. The repository is being prepared for public sharing. Two side-by-side implementations, a committed binary, and migration-process documents make the tree hard to read.
- **Decision:** Remove `src/*.cs`, `build.ps1`, and `dist/LocalPass.exe` from the working tree. Preserve the original migration baseline at tag `wpf-checkpoint` (`e048926`) and the final WPF state at tag `wpf-final` (`e9e5600`). The WPF code remains the historical Windows behavioral reference; consult it by tag, not by path.
- **Rationale:** Windows verification of the Tauri build is sufficient to stop carrying a second implementation. Tags preserve resurrectability without cluttering the tree.
- **Consequences:** This retires LP-006's requirement that WPF stay in the repository tree. LP-006's separation of visual and behavioral authority remains in force. It does **not** declare cross-platform parity: macOS and Linux runtime behavior is still unverified and Wayland clipboard remains unsupported. Parity remains an open evidence gate (see the architecture's "Required parity gates").
- **Partially supersedes:** LP-006 (tree-preservation requirement only)
- **Evidence:** tags `wpf-checkpoint` and `wpf-final`; PRs #1-#3; [authority order](TAURI_ARCHITECTURE.md#authority-order)

## Open items at the close of the source session

This is a historical list, not the current project backlog. These items were not decided on 2026-07-12. Later resolutions must keep their actual decision date:

- Native macOS and Linux build, packaging, signing, and runtime-parity evidence.
- Exact user-facing guarantees for every clipboard environment, especially Wayland.
- Native translucency, focus/collapse, accessibility, work-area, and DPI behavior.
- The parity gate that permits retiring the WPF reference.

Some implementation details have since been specified in [the accepted architecture](TAURI_ARCHITECTURE.md) and [migration plan](https://github.com/wcvessels/local-pass-tool/blob/wpf-final/_design/TAURI_MIGRATION_PLAN.md), but their remaining evidence gates are not converted into “done” by this historical log.
