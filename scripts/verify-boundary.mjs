import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
const failures = [];

function check(condition, message) {
  if (!condition) failures.push(message);
}

function same(actual, expected) {
  return JSON.stringify(actual) === JSON.stringify(expected);
}

function own(value, key) {
  return Object.prototype.hasOwnProperty.call(value, key);
}

function hasKey(value, key) {
  if (Array.isArray(value)) return value.some((item) => hasKey(item, key));
  if (!value || typeof value !== "object") return false;
  return own(value, key) || Object.values(value).some((item) => hasKey(item, key));
}

async function read(relativePath) {
  return readFile(path.join(root, relativePath), "utf8");
}

async function json(relativePath) {
  return JSON.parse(await read(relativePath));
}

async function filesUnder(relativePath) {
  const base = path.join(root, relativePath);
  const entries = await readdir(base, { recursive: true, withFileTypes: true });
  return entries
    .filter((entry) => entry.isFile())
    .map((entry) => path.join(entry.parentPath, entry.name));
}

function tomlSection(source, header) {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === header);
  if (start < 0) return "";
  const end = lines.findIndex((line, index) => index > start && /^\s*\[/.test(line));
  return lines.slice(start + 1, end < 0 ? undefined : end).join("\n");
}

function bracketDepth(value) {
  return [...value].reduce(
    (total, character) =>
      total + (character === "[" || character === "{" ? 1 : character === "]" || character === "}" ? -1 : 0),
    0,
  );
}

function canonicalToml(value) {
  return value.replace(/\s+/g, "").replace(/,([\]}])/g, "$1");
}

function tomlAssignments(section) {
  const entries = new Map();
  const lines = section.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const match = /^\s*([A-Za-z0-9_-]+)\s*=\s*(.*)$/.exec(lines[index]);
    if (!match) continue;
    const name = match[1];
    let value = match[2];
    let depth = bracketDepth(value);
    while (depth > 0 && index + 1 < lines.length) {
      value += lines[(index += 1)].trim();
      depth += bracketDepth(lines[index]);
    }
    entries.set(name, canonicalToml(value));
  }
  return entries;
}

function checkTomlSection(source, header, expected) {
  const actual = tomlAssignments(tomlSection(source, header));
  check(
    same([...actual.keys()].sort(), Object.keys(expected).sort()),
    `${header} must contain only: ${Object.keys(expected).join(", ")}`,
  );
  for (const [name, value] of Object.entries(expected)) {
    check(actual.get(name) === canonicalToml(value), `${header} ${name} configuration changed`);
  }
}

const packageJson = await json("package.json");
check(
  same(packageJson.dependencies, { "@tauri-apps/api": "2.11.1" }),
  "runtime npm dependencies must contain only pinned @tauri-apps/api",
);
check(
  same(packageJson.devDependencies, { "@tauri-apps/cli": "2.11.4", typescript: "5.9.3", vite: "7.3.6" }),
  "development npm dependencies changed",
);
check(
  !Object.keys({
    ...(packageJson.dependencies ?? {}),
    ...(packageJson.devDependencies ?? {}),
  }).some((name) => name.startsWith("@tauri-apps/plugin-")),
  "generic Tauri plugins are forbidden",
);

const expectedPackagingScripts = {
  "tauri:check:native": "tauri build --ci --no-bundle -- --locked",
  "tauri:bundle:windows:unsigned": "tauri build --ci --no-sign --bundles nsis -- --locked",
  "tauri:bundle:macos:unsigned": "tauri build --ci --no-sign --bundles dmg -- --locked",
  "tauri:bundle:linux:unsigned": "tauri build --ci --no-sign --bundles appimage,deb -- --locked",
};
const packagingScripts = packageJson.scripts ?? {};
const actualPackagingNames = Object.keys(packagingScripts).filter((name) => name === "tauri:check:native" || name.startsWith("tauri:bundle:")).sort();
check(same(actualPackagingNames, Object.keys(expectedPackagingScripts).sort()), "native-check and unsigned bundle script names changed");
for (const [name, command] of Object.entries(expectedPackagingScripts)) {
  check(packagingScripts[name] === command, `${name} command changed`);
}
check(!Object.values(packagingScripts).some((command) => /--bundles(?:=|\s+)all\b/i.test(command)), "npm scripts must never target all bundles");
const expectedPlatformBundles = {
  "src-tauri/tauri.windows.conf.json": ["nsis"],
  "src-tauri/tauri.macos.conf.json": ["dmg"],
  "src-tauri/tauri.linux.conf.json": ["appimage", "deb"],
};
const platformConfigs = new Map();
for (const [relativePath, expectedTargets] of Object.entries(expectedPlatformBundles)) {
  const platformConfig = await json(relativePath);
  platformConfigs.set(relativePath, platformConfig);
  check(same(Object.keys(platformConfig).sort(), ["$schema", "bundle"].sort()), `${relativePath} top-level keys changed`);
  check(platformConfig.$schema === "../node_modules/@tauri-apps/cli/config.schema.json", `${relativePath} schema path changed`);
  const expectedBundleKeys = relativePath.endsWith("tauri.windows.conf.json") ? ["active", "targets", "createUpdaterArtifacts", "windows"] : ["active", "targets", "createUpdaterArtifacts"];
  check(same(Object.keys(platformConfig.bundle ?? {}).sort(), expectedBundleKeys.sort()), `${relativePath} bundle keys changed`);
  const targets = platformConfig.bundle?.targets;
  check(platformConfig.bundle?.active === true, `${relativePath} bundling must stay active`);
  check(same(targets, expectedTargets), `${relativePath} bundle targets changed`);
  check(platformConfig.bundle?.createUpdaterArtifacts === false, `${relativePath} updater artifacts must stay disabled`);
  check(targets !== "all" && (!Array.isArray(targets) || !targets.includes("all")), `${relativePath} must never target all bundles`);
}
const windowsBundle = platformConfigs.get("src-tauri/tauri.windows.conf.json")?.bundle;
check(windowsBundle?.windows?.webviewInstallMode?.type === "offlineInstaller", "Windows WebView2 mode must stay offlineInstaller");
check(windowsBundle?.windows?.webviewInstallMode?.silent === true, "Windows WebView2 offline installer must stay silent");
check(same(Object.keys(windowsBundle?.windows ?? {}), ["webviewInstallMode"]), "Windows bundle configuration contains unexpected keys");
check(same(Object.keys(windowsBundle?.windows?.webviewInstallMode ?? {}).sort(), ["silent", "type"]), "Windows WebView2 mode contains unexpected keys");


const expectedCsp =
  "default-src 'self'; connect-src ipc: http://ipc.localhost; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; object-src 'none'; frame-src 'none'; worker-src 'none'; media-src 'none'; base-uri 'none'; form-action 'none'";
const config = await json("src-tauri/tauri.conf.json");
const baseBundleTargets = config.bundle?.targets;
check(baseBundleTargets !== "all" && (!Array.isArray(baseBundleTargets) || !baseBundleTargets.includes("all")), "base Tauri config must never target all bundles");
check(
  same(config.app?.security?.capabilities, ["localpass-main", "localpass-about"]),
  "Tauri config must name only the two LocalPass capabilities",
);
check(config.app?.withGlobalTauri === false, "global Tauri injection must stay disabled");
check(config.app?.security?.assetProtocol?.enable === false, "asset protocol must stay disabled");
check(config.app?.security?.freezePrototype === true, "webview prototypes must stay frozen");
check(same(config.app?.security?.assetProtocol?.scope, []), "asset protocol scope must stay empty");
check(config.app?.security?.csp === expectedCsp, "production CSP changed");
check(config.app?.security?.devCsp === expectedCsp, "development CSP changed");
check(config.bundle?.createUpdaterArtifacts === false, "updater artifacts must stay disabled");
check(config.build?.frontendDist === "../dist-ui", "frontendDist must remain the local dist-ui directory");
check(
  config.build?.devUrl === "http://127.0.0.1:1420",
  "development URL must stay fixed to loopback port 1420",
);
const windows = config.app?.windows;
check(Array.isArray(windows) && windows.length === 2, "Tauri config must define exactly two windows");
check(same(windows?.map((window) => window.label), ["main", "about"]), "window labels must be exactly main and about");
const expectedWindowUrls = {
  main: "index.html",
  about: "index.html?view=about",
};
const expectedWindowFlags = {
  create: false,
  incognito: true,
  generalAutofillEnabled: false,
  allowLinkPreview: false,
  dragDropEnabled: false,
  devtools: false,
  browserExtensionsEnabled: false,
  zoomHotkeysEnabled: false,
};
for (const [label, url] of Object.entries(expectedWindowUrls)) {
  const window = windows?.find((candidate) => candidate.label === label);
  check(window?.url === url, `${label} window URL must stay ${url}`);
  for (const [flag, expected] of Object.entries(expectedWindowFlags)) {
    check(window?.[flag] === expected, `${label} window ${flag} must stay ${expected}`);
  }
}

const vite = await read("vite.config.ts");
check((vite.match(/\bhost\s*:/g) ?? []).length === 1 && /\bhost\s*:\s*["']127\.0\.0\.1["']/.test(vite), "Vite host must be exactly 127.0.0.1");
check((vite.match(/\bport\s*:/g) ?? []).length === 1 && /\bport\s*:\s*1420\b/.test(vite), "Vite port must be exactly 1420");
check((vite.match(/\bstrictPort\s*:/g) ?? []).length === 1 && /\bstrictPort\s*:\s*true\b/.test(vite), "Vite strictPort must stay true");
check((vite.match(/\bhmr\s*:/g) ?? []).length === 1 && /\bhmr\s*:\s*false\b/.test(vite), "Vite HMR must stay disabled");

const expectedCapabilities = {
  "src-tauri/capabilities/localpass-main.json": [
    "allow-generate-passwords",
    "allow-copy-password",
    "allow-clear-sensitive-state",
    "allow-clipboard-status",
    "allow-set-window-view",
    "allow-set-pointer-inside",
    "allow-set-always-on-top",
    "allow-start-window-drag",
    "allow-open-about",
    "allow-redaction-ack",
    "allow-request-close",
  ],
  "src-tauri/capabilities/localpass-about.json": [
    "allow-clipboard-status",
    "allow-set-clipboard-timeout",
    "allow-set-macos-best-effort-clear",
    "allow-set-pointer-inside",
    "allow-start-window-drag",
    "allow-close-about",
  ],
};
const expectedCapabilityIdentity = {
  "src-tauri/capabilities/localpass-main.json": {
    identifier: "localpass-main",
    windows: ["main"],
  },
  "src-tauri/capabilities/localpass-about.json": {
    identifier: "localpass-about",
    windows: ["about"],
  },
};

for (const [relativePath, expected] of Object.entries(expectedCapabilities)) {
  const capability = await json(relativePath);
  const identity = expectedCapabilityIdentity[relativePath];
  check(capability.identifier === identity.identifier, `${relativePath} identifier changed`);
  check(capability.local === true, `${relativePath} must stay local`);
  check(same(capability.windows, identity.windows), `${relativePath} window labels changed`);
  check(
    same(capability.permissions, expected),
    `${relativePath} permissions changed`,
  );
  check(!own(capability, "remote"), `${relativePath} must not define remote URLs`);
  check(!hasKey(capability, "scope"), `${relativePath} must not define a permission scope`);
  check(
    Array.isArray(capability.permissions) &&
      capability.permissions.every((permission) => typeof permission === "string" && !permission.includes("*") && !permission.startsWith("core:")),
    `${relativePath} must not contain wildcards or core defaults`,
  );
}

const cargo = await read("src-tauri/Cargo.toml");
const dependencyHeaders = cargo
  .split(/\r?\n/)
  .map((line) => line.trim())
  .filter((line) => /^\[.*dependencies\]$/.test(line));
const expectedDependencyHeaders = [
  "[build-dependencies]",
  "[dependencies]",
  "[target.'cfg(target_os = \"windows\")'.dependencies]",
  "[target.'cfg(target_os = \"macos\")'.dependencies]",
  "[target.'cfg(target_os = \"linux\")'.dependencies]",
];
check(same(dependencyHeaders, expectedDependencyHeaders), "Cargo dependency sections changed");

checkTomlSection(cargo, "[build-dependencies]", {
  "tauri-build": "{version=\"=2.6.3\",features=[]}",
});
checkTomlSection(cargo, "[dependencies]", {
  getrandom: "\"=0.3.3\"",
  serde: "{version=\"=1.0.219\",features=[\"derive\"]}",
  tauri: "{version=\"=2.11.5\",default-features=false,features=[\"common-controls-v6\",\"compression\",\"dbus\",\"macos-private-api\",\"wry\",\"x11\"]}",
  "tauri-plugin-single-instance": "\"=2.4.2\"",
  zeroize: "{version=\"=1.8.1\",features=[\"alloc\"]}",
});

checkTomlSection(cargo, "[target.'cfg(target_os = \"windows\")'.dependencies]", {
  "windows-sys": "{version=\"=0.61.2\",features=[\"Win32_Foundation\",\"Win32_System_DataExchange\",\"Win32_System_Memory\",\"Win32_UI_WindowsAndMessaging\"]}",
});
checkTomlSection(cargo, "[target.'cfg(target_os = \"macos\")'.dependencies]", {
  "objc2-app-kit": "{version=\"=0.3.2\",default-features=false,features=[\"std\",\"NSPasteboard\"]}",
  "objc2-foundation": "{version=\"=0.3.2\",default-features=false,features=[\"std\",\"NSData\",\"NSString\"]}",
});
checkTomlSection(cargo, "[target.'cfg(target_os = \"linux\")'.dependencies]", {
  x11rb: "{version=\"=0.13.2\",default-features=false}",
});
checkTomlSection(cargo, "[features]", {
  default: "[]",
  "custom-protocol": "[\"tauri/custom-protocol\"]",
});

const rustFiles = [...(await filesUnder("src-tauri/src")), path.join(root, "src-tauri", "build.rs")];
const rustSource = (await Promise.all(rustFiles.map((filePath) => readFile(filePath, "utf8")))).join("\n");
const forbiddenNative = [
  [/\b(?:std|tokio|async_std)::net\b/, "native network namespace"],
  [/\b(?:TcpStream|TcpListener|UdpSocket|ToSocketAddrs)\b/, "native socket API"],
  [/\b(?:reqwest|ureq|hyper|curl|tungstenite|isahc|attohttpc|surf)::/, "native network client"],
  [/\b(?:std|tokio|async_std)::fs\b/, "native filesystem namespace"],
  [/\b(?:File|OpenOptions)::(?:open|create|new)\s*\(/, "native filesystem API"],
  [/\bstd::process\b|\bCommand::(?:new|spawn|output)\s*\(/, "native process API"],
  [/\btauri_plugin_(?:clipboard|http|shell|fs|opener|updater|store|log|process)\b/, "forbidden Tauri plugin API"],
  [/\b(?:telemetry|analytics|sentry|posthog|opentelemetry)\b/i, "native telemetry API"],
  [/powershell|cmd\.exe|\/bin\/(?:sh|bash)|osascript|xdg-open|wl-copy|xclip|xsel/i, "shell executable"],
];
for (const [pattern, label] of forbiddenNative) {
  check(!pattern.test(rustSource), `${label} found in native Rust source`);
}

const hardeningTokens = [
  ["HARDENING_SCRIPT", "document-start hardening script"],
  ["'copy'", "copy event denial"],
  ["'cut'", "cut event denial"],
  ["'dragstart'", "drag event denial"],
  ["'contextmenu'", "context-menu denial"],
  ["'beforeprint'", "print event denial"],
  ["execCommand", "legacy clipboard denial"],
  ["navigator, 'clipboard'", "browser clipboard denial"],
  ["serviceWorker", "service-worker denial"],
  ["window, 'open'", "window.open denial"],
  ["window, 'print'", "window.print denial"],
  [".initialization_script(", "hardened initialization hook"],
  [".on_navigation(", "navigation denial hook"],
  [".on_new_window(", "new-window denial hook"],
  ["NewWindowResponse::Deny", "new-window deny response"],
  [".on_download(", "download denial hook"],
  ["allowed_navigation", "navigation allowlist"],
  ["tauri.localhost", "embedded Windows origin allowlist"],
  ["127.0.0.1", "debug loopback origin allowlist"],
];
for (const [token, label] of hardeningTokens) {
  check(rustSource.includes(token), `missing ${label} in native Rust source`);
}
const hardeningPatterns = [
  [/\.initialization_script\s*\(\s*HARDENING_SCRIPT\s*\)/, "hardening script installation"],
  [/\.on_navigation\s*\(\s*allowed_navigation\s*\)/, "navigation allowlist installation"],
  [/\.on_new_window\s*\(\s*\|[^|]*\|\s*NewWindowResponse::Deny\s*\)/, "new-window denial"],
  [/\.on_download\s*\(\s*\|[^|]*\|\s*false\s*\)/, "download denial"],
];
for (const [pattern, label] of hardeningPatterns) {
  check(pattern.test(rustSource), `missing ${label} in native Rust source`);
}

const forbiddenRuntime = [
  [/(?:https?|wss?|ftp):\/\//i, "remote URL"],
  [/\bfetch\s*\(/i, "fetch"],
  [/\bWebSocket\b/i, "WebSocket"],
  [/\bEventSource\b/i, "EventSource"],
  [/\bXMLHttpRequest\b/i, "XMLHttpRequest"],
  [/\bsendBeacon\b/i, "sendBeacon"],
  [/\bRTCPeerConnection\b/i, "WebRTC"],
  [/\bWebTransport\b/i, "WebTransport"],
  [/navigator\.clipboard/i, "browser clipboard"],
  [/\bClipboardItem\b/i, "ClipboardItem"],
  [/serviceWorker\.register/i, "service worker"],
  [/\bcaches\b|\bCacheStorage\b/i, "Cache Storage"],
  [/document\.cookie|\bcookieStore\b/i, "browser cookies"],
  [/\bshowOpenFilePicker\b|\bshowSaveFilePicker\b/i, "browser file picker"],
  [/\bFileSystem(?:File|Directory)?Handle\b/i, "browser filesystem handle"],
  [/\bSharedWorker\b/i, "SharedWorker"],
  [/\blocalStorage\b/i, "localStorage"],
  [/\bsessionStorage\b/i, "sessionStorage"],
  [/\bindexedDB\b/i, "indexedDB"],
];

const runtimeFiles = [
  path.join(root, "index.html"),
  ...(await filesUnder("frontend")),
  ...(await filesUnder("dist-ui")),
];
for (const filePath of runtimeFiles) {
  const source = await readFile(filePath, "utf8");
  for (const [pattern, label] of forbiddenRuntime) {
    check(!pattern.test(source), `${label} found in ${path.relative(root, filePath)}`);
  }
}

if (failures.length > 0) {
  for (const failure of failures) process.stderr.write(`BOUNDARY FAIL: ${failure}\n`);
  process.exitCode = 1;
} else {
  process.stdout.write("LocalPass boundary checks passed.\n");
}
