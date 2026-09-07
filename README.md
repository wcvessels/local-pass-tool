# LocalPass

Compact, local-only Windows password generator. Double-click
[`dist\LocalPass.exe`](dist/LocalPass.exe). No terminal needed for normal use.

## Use

- Choose a length from 4 to 64 and a count from 1 to 99.
- Toggle lowercase, uppercase, numbers, symbols, and **NO 0O1l**.
- Press **GENERATE**, then **COPY** beside the password you need.
- Use **✱** to mask all results; click a masked result to reveal that row.
- Use **?** to change the clipboard release timer from 5 to 60 seconds.
- Press **CLEAR** to remove generated values and release clipboard content owned by LocalPass.
- **✕** closes LocalPass and releases clipboard content it still owns.
- Clicking outside LocalPass collapses it immediately. Hover never expands it.
- Click the collapsed bar, or focus it and press `Enter`/`Space`, to expand. In Tauri, press and drag the bar to move without expanding. Its separate **✕** closes safely while collapsed.
- Additional launches exit while one LocalPass instance is already running.

Shortcuts: `Ctrl+G` generates; `Ctrl+L` clears. Close the app with `Alt+F4`
or its taskbar command.

## Appearance

- Compact 400px frosted-glass widget with 90% expanded surfaces in dark and light themes. Expanded and About shells are fully visible while focused, 95% while inactive-hovered, and 85% while inactive-idle. About controls collapsed opacity from 25–75% in 5% steps; the session-only default is 50%.
- Main, About, and collapsed surfaces are shadow-free. In Tauri, the collapsed rail keeps the expanded width and scales above 100% app zoom: 400×40 at 75–100%, up to 800×80 at 200%, capped by the monitor work area. Rail text and Close scale with it.
- Results use a fixed 11px monospace size, stay on one line, and ellipsize before the **COPY** gutter. **COPY** always uses the complete password.
- The movable About pane opens 14px to the right, left, below, or above as screen space permits.
- Windows High Contrast uses opaque system colors and never auto-collapses.

Theme and window choices are session-only. Nothing is saved.

## Security boundary

- Windows cryptographic randomness with unbiased selection and Fisher–Yates shuffle.
- Every generated password contains at least one character from each enabled group.
- Memory only: no files, settings, logs, analytics, telemetry, server, or network calls.
- Clipboard changes happen only after an explicit per-row **COPY**.
- Copied text is marked for exclusion from Windows Clipboard History and cloud clipboard sync.
- Owned clipboard content is released after the selected timeout, on **CLEAR**, regeneration, or normal close.
- Clipboard cleanup never erases content another application placed there.
- Generated text masks before the window loses focus.

Windows and third-party applications can still read the live clipboard. Endpoint
malware, screen capture, third-party clipboard managers, and forensic memory
recovery are outside this utility's boundary. Managed UI strings cannot be
guaranteed physically overwritten in RAM.

## Rebuild

Normal use never invokes PowerShell. Developers can rebuild with the
Windows-provided .NET Framework compiler:

```powershell
.\build.ps1
```

No package restore or internet connection is used.
