# LocalPass

Compact, local-only Windows password generator. Double-click
[`dist\LocalPass.exe`](dist/LocalPass.exe). No terminal needed for normal use.

## Use

- Choose a length from 8 to 64.
- Enter a count from 1 to 50. Up and Down adjust the count.
- Toggle lowercase, uppercase, numbers, symbols, and **No look-alikes**.
- Press **Generate**, then use the **Copy** button beside the password you need.
- Press **Clear** to remove generated values and release clipboard content owned by LocalPass.

Shortcuts: `Ctrl+G` generates; `Ctrl+L` clears.

## Appearance

- Gunmetal dark frost and white-grey light frost; the theme button switches them.
- Active window: 90% opacity.
- Inactive hover: 72% opacity with passwords masked.
- Inactive pointer-out: 22% opacity, then settings/results roll into the header.
- Hovering or activating the header unrolls the utility.
- Windows High Contrast disables transparency.

Theme and window choices are session-only. Nothing is saved.

On Windows 10, LocalPass uses a supported translucent frosted-tint fallback.
Supported system backdrop acrylic requires newer Windows 11; LocalPass avoids
undocumented composition hooks.

## Security boundary

- Windows cryptographic randomness with unbiased selection.
- Every generated password contains at least one character from each enabled group.
- Batch values are unique.
- Memory only: no files, settings, logs, analytics, telemetry, server, or network calls.
- Clipboard changes happen only after an explicit per-row **Copy**.
- Copied text is marked for exclusion from Windows Clipboard History and cloud clipboard sync.
- Owned clipboard content is released after 30 seconds, on **Clear**, regeneration, or normal close.
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
