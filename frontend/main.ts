import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const hardenedWindow = window as Window & { readonly __LOCALPASS_HARDENING_FAILED__?: boolean };
if (hardenedWindow.__LOCALPASS_HARDENING_FAILED__ === true) {
  document.documentElement.replaceChildren();
  throw new Error("LocalPass browser hardening failed.");
}

type AppView = "main" | "about";
type Theme = "dark" | "light";
type Tone = "neutral" | "success" | "warning" | "error";
type GroupKey = "lowercase" | "uppercase" | "numbers" | "symbols" | "excludeAmbiguous";

type GeneratedBatch = {
  batch_id: string;
  passwords: string[];
  view_epoch: number;
};

type ClipboardStatus = {
  state: "idle" | "policy_off" | "countdown" | "release_pending";
  policy: string;
  deadline_ms: number | null;
  timeout_seconds: number;
  macos_best_effort_clear: boolean;
  release_sequence: number;
  last_release: string | null;
};

const view: AppView = new URLSearchParams(location.search).get("view") === "about" ? "about" : "main";
const root = document.documentElement;
const statusTimers = new WeakMap<HTMLElement, number>();
const zoomSteps = [75, 90, 100, 110, 125, 150, 175, 200] as const;
let zoomIndex = 2;


window.addEventListener("localpass:window-opacity", (event) => {
  const detail = (event as CustomEvent<unknown>).detail;
  if (detail === "active" || detail === "hovered" || detail === "idle") {
    root.dataset.windowOpacity = detail;
  }
});
function required<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLElement)) {
    throw new Error("Missing required UI element.");
  }
  return element as T;
}

function commandCode(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object") {
    const record = error as Record<string, unknown>;
    if (typeof record.code === "string") return record.code;
    if (typeof record.message === "string") return record.message;
  }
  return "unknown";
}

function isCode(error: unknown, code: string): boolean {
  return commandCode(error).toLowerCase().includes(code.toLowerCase());
}

function setStatus(element: HTMLElement, message: string, tone: Tone = "neutral", timeout = 0): void {
  const activeTimer = statusTimers.get(element);
  if (activeTimer !== undefined) window.clearTimeout(activeTimer);
  element.textContent = message;
  element.dataset.tone = tone;
  element.hidden = message.length === 0;
  if (timeout > 0) {
    const timer = window.setTimeout(() => {
      element.textContent = "";
      element.hidden = true;
      statusTimers.delete(element);
    }, timeout);
    statusTimers.set(element, timer);
  }
}

function parseClipboardStatus(value: unknown): ClipboardStatus | null {
  if (!value || typeof value !== "object") return null;
  const item = value as Record<string, unknown>;
  const deadline = item.deadline_ms;
  const lastRelease = item.last_release;
  if (
    typeof item.state !== "string" ||
    typeof item.policy !== "string" ||
    typeof item.timeout_seconds !== "number" ||
    typeof item.macos_best_effort_clear !== "boolean" ||
    typeof item.release_sequence !== "number" ||
    !Number.isSafeInteger(item.release_sequence) ||
    item.release_sequence < 0
  ) return null;
  if (deadline !== null && typeof deadline !== "number") return null;
  if (lastRelease !== undefined && lastRelease !== null && typeof lastRelease !== "string") return null;
  return {
    state: item.state as ClipboardStatus["state"],
    policy: item.policy,
    deadline_ms: deadline,
    timeout_seconds: item.timeout_seconds,
    macos_best_effort_clear: item.macos_best_effort_clear,
    release_sequence: item.release_sequence,
    last_release: typeof lastRelease === "string" ? lastRelease : null,
  };
}

function policyReady(status: ClipboardStatus | null): boolean {
  return (
    status !== null &&
    status.policy !== "not_implemented" &&
    status.policy !== "unsupported"
  );
}

function releaseOutcomeStatus(outcome: string | null): { message: string; tone: Tone } | null {
  switch (outcome) {
    case "cleared":
      return { message: "Clipboard cleared by LocalPass.", tone: "success" };
    case "ownership_lost":
      return {
        message: "Clipboard changed before release. LocalPass did not clear the newer value.",
        tone: "neutral",
      };
    case "suppressed_by_policy":
      return {
        message: "Clipboard release was suppressed by policy. Copied content remains until replaced.",
        tone: "warning",
      };
    case "unsupported":
      return { message: "Clipboard release is unsupported on this platform.", tone: "warning" };
    case "busy":
      return {
        message: "Clipboard remained busy. LocalPass could not confirm release; copied content may remain until replaced.",
        tone: "warning",
      };
    case "fatal":
      return { message: "Clipboard release failed.", tone: "error" };
    default:
      return null;
  }
}
function isMacOS(): boolean {
  const platform = navigator.platform.toLowerCase();
  const agent = navigator.userAgent.toLowerCase();
  return platform.includes("mac") || agent.includes("mac os");
}

function installShortcutGuards(status: HTMLElement, onGenerate?: () => void, onClear?: () => void): void {
  window.addEventListener("keydown", (event) => {
    if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
    const key = event.key.toLowerCase();
    if (key === "g" && onGenerate) {
      event.preventDefault();
      onGenerate();
      return;
    }
    if (key === "l" && onClear) {
      event.preventDefault();
      onClear();
      return;
    }
    if (key === "p" || key === "s" || key === "u") {
      event.preventDefault();
      setStatus(status, "That browser command is disabled in LocalPass.", "warning", 2600);
      return;
    }
    if (key === "0") {
      event.preventDefault();
      zoomIndex = 2;
      applyZoom(status);
      return;
    }
    if (key === "+" || key === "=") {
      event.preventDefault();
      zoomIndex = Math.min(zoomSteps.length - 1, zoomIndex + 1);
      applyZoom(status);
      return;
    }
    if (key === "-") {
      event.preventDefault();
      zoomIndex = Math.max(0, zoomIndex - 1);
      applyZoom(status);
    }
  });
}

function applyZoom(status: HTMLElement): void {
  root.dataset.zoom = String(zoomSteps[zoomIndex]);
  setStatus(status, "Zoom " + zoomSteps[zoomIndex] + "%", "neutral", 1600);
  window.dispatchEvent(new CustomEvent("localpass:layout-changed"));
}

function passwordFitClass(length: number): string {
  const rawPixels = (280 / Math.max(1, length) - 0.6) / 0.62;
  const pixels = Math.max(11, Math.min(16, rawPixels));
  if (rawPixels < 9) return "password-fit-xxs password-display--wrap";
  if (pixels >= 15.5) return "password-fit-lg";
  if (pixels >= 13.5) return "password-fit-md";
  if (pixels >= 12) return "password-fit-sm";
  return "password-fit-xs";
}

function queryTheme(): Theme {
  return new URLSearchParams(location.search).get("theme") === "light" ? "light" : "dark";
}

root.dataset.view = view;
root.dataset.zoom = "100";

if (view === "main") {
  void startMain();
} else {
  void startAbout();
}

async function startMain(): Promise<void> {
  const mainView = required<HTMLElement>("main-view");
  const aboutView = required<HTMLElement>("about-view");
  const shell = required<HTMLElement>("main-shell");
  const expandedContent = required<HTMLElement>("expanded-content");
  const rolledContent = required<HTMLElement>("rolled-content");
  const expandButton = required<HTMLButtonElement>("expand-button");
  const rolledCloseButton = required<HTMLButtonElement>("rolled-close-button");
  const dragRegion = required<HTMLElement>("drag-region");
  const aboutButton = required<HTMLButtonElement>("about-button");
  const maskButton = required<HTMLButtonElement>("mask-button");
  const pinButton = required<HTMLButtonElement>("pin-button");
  const themeButton = required<HTMLButtonElement>("theme-button");
  const closeButton = required<HTMLButtonElement>("close-button");
  const lengthInput = required<HTMLInputElement>("length-input");
  const lengthRange = required<HTMLInputElement>("length-range");
  const countInput = required<HTMLInputElement>("count-input");
  const generateButton = required<HTMLButtonElement>("generate-button");
  const clearButton = required<HTMLButtonElement>("clear-button");
  const resultsSection = required<HTMLElement>("results-section");
  const resultsList = required<HTMLElement>("results-list");
  const statusElement = required<HTMLElement>("main-status");
  const highContrastMedia = window.matchMedia("(forced-colors: active)");

  const state = {
    theme: "dark" as Theme,
    masked: false,
    active: document.hasFocus(),
    pinned: true,
    rolled: false,
    length: 20,
    count: 3,
    groups: {
      lowercase: true,
      uppercase: true,
      numbers: true,
      symbols: true,
      excludeAmbiguous: true,
    } as Record<GroupKey, boolean>,
    passwords: [] as string[],
    revealed: new Set<number>(),
    batchId: null as string | null,
    copiedIndex: null as number | null,
    generationOperation: null as number | null,
    copyOperation: null as number | null,
    clipboardStatus: null as ClipboardStatus | null,
    viewEpoch: Date.now(),
    operation: 0,
    clipboardTimer: 0,
    clipboardRefreshInFlight: false,
    clipboardRefreshQueued: false,
    clipboardStatusEpoch: 0,
    lastSeenReleaseSequence: 0,
    lastReportedHeight: 0,
    layoutFrame: 0,
  };

  aboutView.hidden = true;
  mainView.hidden = false;
  applyTheme();
  applyPressedStates();
  bindNumeric(lengthInput, 4, 64, 20, (value) => {
    state.length = value;
    lengthRange.value = String(value);
  });
  bindNumeric(countInput, 1, 99, 1, (value) => {
    state.count = value;
  });

  lengthRange.addEventListener("input", () => {
    state.length = Number(lengthRange.value);
    lengthInput.value = String(state.length);
  });

  document.querySelectorAll<HTMLButtonElement>(".group-button").forEach((button) => {
    button.addEventListener("click", () => {
      const key = button.dataset.group as GroupKey | undefined;
      if (!key || !(key in state.groups)) return;
      state.groups[key] = !state.groups[key];
      button.setAttribute("aria-pressed", String(state.groups[key]));
    });
  });

  generateButton.addEventListener("click", () => void generate());
  clearButton.addEventListener("click", () => void clearResults("user"));
  maskButton.addEventListener("click", () => {
    state.masked = !state.masked;
    if (!state.masked) state.revealed.clear();
    applyPressedStates();
    renderResults();
  });
  themeButton.addEventListener("click", () => {
    state.theme = state.theme === "dark" ? "light" : "dark";
    applyTheme();
  });
  pinButton.addEventListener("click", () => void setPinned(!state.pinned));
  aboutButton.addEventListener("click", () => void openAbout());
  closeButton.addEventListener("click", () => void requestClose());
  rolledCloseButton.addEventListener("click", () => void requestClose());
  expandButton.addEventListener("click", () => void requestWindowView("expanded"));

  dragRegion.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) return;
    if (event.target instanceof Element && event.target.closest("button, input")) return;
    void invoke("start_window_drag").catch(() => {
      setStatus(statusElement, "Window drag is not available yet.", "warning", 2400);
    });
  });

  shell.addEventListener("pointerenter", () => {
    void invoke("set_pointer_inside", { inside: true }).catch(() => undefined);
  });
  shell.addEventListener("pointerleave", () => {
    void invoke("set_pointer_inside", { inside: false }).catch(() => undefined);
  });

  window.addEventListener("focus", () => {
    state.active = true;
    renderResults();
  });
  window.addEventListener("blur", () => {
    state.active = false;
    state.revealed.clear();
    renderResults();
  });
  window.addEventListener("resize", reportContentHeight);
  window.addEventListener("localpass:layout-changed", reportContentHeight);
  window.addEventListener("localpass:mask-all", () => {
    state.active = false;
    state.revealed.clear();
    renderResults();
  });
  window.addEventListener("localpass:window-view", (event) => {
    const detail = (event as CustomEvent<unknown>).detail;
    if (detail === "rolled" || detail === "expanded") applyWindowView(detail);
  });
  window.addEventListener("localpass:clipboard-status-changed", () => {
    state.clipboardStatusEpoch += 1;
    void refreshClipboardStatus(false);
  });
  window.addEventListener("localpass:native-close", (event) => {
    const ticket = (event as CustomEvent<unknown>).detail;
    if (!Number.isSafeInteger(ticket) || Number(ticket) <= 0) return;
    try {
      redactResults();
    } catch {
      setStatus(statusElement, "Passwords redacted; close state could not advance.", "warning");
      return;
    }
    void invoke("redaction_ack", { closeTicket: ticket }).catch(() => undefined);
  });
  highContrastMedia.addEventListener("change", () => {
    void requestWindowView("expanded", false);
  });
  window.addEventListener("beforeunload", scrubBeforeUnload);
  document.addEventListener("contextmenu", (event) => {
    if (event.target instanceof Element && event.target.closest(".password-display")) event.preventDefault();
  });
  document.addEventListener("copy", (event) => event.preventDefault());
  document.addEventListener("cut", (event) => event.preventDefault());
  document.addEventListener("dragstart", (event) => {
    if (event.target instanceof Element && event.target.closest(".password-display")) event.preventDefault();
  });

  installShortcutGuards(statusElement, () => void generate(), () => void clearResults("user"));
  const layoutObserver = new ResizeObserver(reportContentHeight);
  layoutObserver.observe(shell);
  await refreshClipboardStatus(true);
  reportContentHeight();

  function bindNumeric(
    input: HTMLInputElement,
    min: number,
    max: number,
    fallback: number,
    commit: (value: number) => void,
  ): void {
    let accepted = input.value;
    input.addEventListener("beforeinput", (event) => {
      const inputEvent = event as InputEvent;
      if (inputEvent.data !== null && !/^[0-9]+$/.test(inputEvent.data)) event.preventDefault();
    });
    input.addEventListener("input", () => {
      if (!/^[0-9]*$/.test(input.value)) {
        input.value = accepted;
        return;
      }
      accepted = input.value;
    });
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        input.blur();
      } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        event.preventDefault();
        const parsed = input.value === "" ? fallback : Number(input.value);
        const next = Math.max(min, Math.min(max, parsed + (event.key === "ArrowUp" ? 1 : -1)));
        input.value = String(next);
        accepted = input.value;
        commit(next);
      }
    });
    input.addEventListener("blur", () => {
      const parsed = input.value === "" ? fallback : Number(input.value);
      const value = Math.max(min, Math.min(max, Number.isFinite(parsed) ? parsed : fallback));
      input.value = String(value);
      accepted = input.value;
      commit(value);
    });
  }

  function applyTheme(): void {
    root.dataset.theme = state.theme;
    themeButton.setAttribute("aria-label", "Switch to " + (state.theme === "dark" ? "light" : "dark") + " theme");
    themeButton.title = themeButton.getAttribute("aria-label") || "Switch theme";
  }

  function applyPressedStates(): void {
    maskButton.setAttribute("aria-pressed", String(state.masked));
    const maskAction = state.masked ? "Show all passwords" : "Mask passwords";
    maskButton.setAttribute("aria-label", maskAction);
    maskButton.title = maskAction;
    pinButton.setAttribute("aria-pressed", String(state.pinned));
  }

  function renderResults(): void {
    resultsList.replaceChildren();
    const hasResults = state.passwords.length > 0;
    resultsSection.hidden = !hasResults;
    clearButton.disabled = !hasResults;
    if (!hasResults) {
      reportContentHeight();
      return;
    }
    const fitClass = passwordFitClass(state.length);
    state.passwords.forEach((password, index) => {
      const number = String(index + 1).padStart(2, "0");
      const visible = state.active && (!state.masked || state.revealed.has(index));
      const row = document.createElement("div");
      row.className = "password-row";
      row.setAttribute("role", "listitem");
      row.setAttribute("aria-label", "Password " + number);

      const indexLabel = document.createElement("span");
      indexLabel.className = "password-index mono";
      indexLabel.textContent = number;

      const display = document.createElement("button");
      display.type = "button";
      display.className = "password-display mono " + fitClass;
      display.draggable = false;
      const canToggleMask = state.active && state.masked;
      display.disabled = !canToggleMask;
      const displayLabel = canToggleMask
        ? (visible ? "Mask password " : "Reveal password ") + number
        : "Password " + number + (visible ? ", visible" : ", hidden");
      display.setAttribute("aria-label", displayLabel);
      const text = document.createElement("span");
      text.className = "password-text";
      text.setAttribute("aria-hidden", "true");
      text.draggable = false;
      text.append(document.createTextNode(visible ? password : "\u2022".repeat(password.length)));
      display.append(text);
      display.addEventListener("click", () => {
        if (!state.active || !state.masked) return;
        if (state.revealed.has(index)) state.revealed.delete(index);
        else state.revealed.add(index);
        renderResults();
      });

      const copy = document.createElement("button");
      copy.type = "button";
      copy.className = "copy-button mono";
      copy.textContent = state.copiedIndex === index ? "COPIED" : "COPY";
      copy.disabled = state.copyOperation !== null || !policyReady(state.clipboardStatus) || !state.batchId;
      copy.setAttribute("aria-label", "Copy password " + number);
      copy.addEventListener("click", () => void copyPassword(index));

      row.append(indexLabel, display, copy);
      resultsList.append(row);
    });
    reportContentHeight();
  }

  function redactResults(): number {
    state.clipboardStatusEpoch += 1;
    state.operation += 1;
    resultsList.querySelectorAll<HTMLElement>(".password-text").forEach((element) => {
      element.textContent = "";
    });
    resultsList.replaceChildren();
    state.passwords.fill("");
    state.passwords = [];
    state.batchId = null;
    state.revealed.clear();
    state.copiedIndex = null;
    resultsSection.hidden = true;
    clearButton.disabled = true;
    stopClipboardTimer();
    state.generationOperation = null;
    state.copyOperation = null;
    generateButton.disabled = false;
    generateButton.textContent = "GENERATE";
    state.viewEpoch = nextViewEpoch(state.viewEpoch);
    return state.operation;
  }

  function nextViewEpoch(epoch: number): number {
    if (!Number.isSafeInteger(epoch) || epoch >= Number.MAX_SAFE_INTEGER) {
      throw new Error("View epoch exhausted.");
    }
    return epoch + 1;
  }

  async function generate(): Promise<void> {
    if (state.generationOperation !== null) return;
    if (!state.groups.lowercase && !state.groups.uppercase && !state.groups.numbers && !state.groups.symbols) return;
    normalizeInputs();
    let operation: number;
    try {
      operation = redactResults();
    } catch {
      setStatus(statusElement, "Generation stopped because the view state could not advance.", "error");
      return;
    }
    state.generationOperation = operation;
    clearButton.disabled = false;
    generateButton.disabled = true;
    generateButton.textContent = "GENERATING\u2026";
    setStatus(statusElement, "Generating locally\u2026");
    try {
      const value = await invoke<unknown>("generate_passwords", {
        request: {
          count: state.count,
          length: state.length,
          lowercase: state.groups.lowercase,
          uppercase: state.groups.uppercase,
          numbers: state.groups.numbers,
          symbols: state.groups.symbols,
          exclude_ambiguous: state.groups.excludeAmbiguous,
          redacted_view_epoch: state.viewEpoch,
        },
      });
      if (operation !== state.operation) return;
      const batch = validateBatch(value);
      if (!batch) {
        await rejectInvalidBatch();
        return;
      }
      state.batchId = batch.batch_id;
      state.passwords = batch.passwords;
      renderResults();
      setStatus(statusElement, "Generated on this device.", "success", 2200);
    } catch (error) {
      if (operation !== state.operation) return;
      const message = isCode(error, "no_character_groups")
        ? "Select at least one character group."
        : isCode(error, "entropy_unavailable")
          ? "Secure system randomness is unavailable. No passwords were generated."
          : isCode(error, "stale_view_epoch")
            ? "Generation was superseded by a newer action."
            : "Password generation failed. No passwords were retained.";
      setStatus(statusElement, message, "error");
    } finally {
      if (state.generationOperation === operation) {
        state.generationOperation = null;
        generateButton.disabled = false;
        clearButton.disabled = state.passwords.length === 0;
        generateButton.textContent = "GENERATE";
      }
    }
  }

  function normalizeInputs(): void {
    lengthInput.blur();
    countInput.blur();
  }

  function validateBatch(value: unknown): GeneratedBatch | null {
    if (!value || typeof value !== "object") return null;
    const batch = value as Record<string, unknown>;
    if (
      typeof batch.batch_id !== "string" ||
      !Array.isArray(batch.passwords) ||
      batch.view_epoch !== state.viewEpoch ||
      batch.passwords.length !== state.count
    ) return null;
    if (!batch.passwords.every((item) => typeof item === "string" && item.length === state.length)) return null;
    return batch as GeneratedBatch;
  }

  async function rejectInvalidBatch(): Promise<void> {
    let epoch: number;
    try {
      redactResults();
      epoch = state.viewEpoch;
    } catch {
      setStatus(statusElement, "Invalid native response was rejected.", "error");
      return;
    }
    setStatus(statusElement, "Invalid native response was rejected and redacted.", "error");
    await invoke("clear_sensitive_state", { reason: "user", redactedViewEpoch: epoch }).catch(() => undefined);
  }

  async function clearResults(reason: "user" | "regenerate" | "close" | "session_ending"): Promise<void> {
    if (
      state.passwords.length === 0 &&
      state.batchId === null &&
      state.generationOperation === null &&
      state.copyOperation === null
    ) return;
    let epoch: number;
    try {
      redactResults();
      epoch = state.viewEpoch;
    } catch {
      setStatus(statusElement, "Local display redaction failed.", "error");
      return;
    }
    const statusEpoch = state.clipboardStatusEpoch;
    setStatus(statusElement, "Passwords removed from this window. Releasing clipboard\u2026");
    try {
      const parsed = parseClipboardStatus(
        await invoke<unknown>("clear_sensitive_state", { reason, redactedViewEpoch: epoch }),
      );
      if (!parsed) throw new Error("invalid_status");
      if (statusEpoch !== state.clipboardStatusEpoch) return;
      state.clipboardStatus = parsed;
      presentClipboardStatus(parsed);
    } catch (error) {
      if (statusEpoch !== state.clipboardStatusEpoch) return;
      const message = isCode(error, "not_implemented")
        ? "Passwords were removed from this window. Native clipboard release is not available yet."
        : "Passwords were removed from this window. Clipboard release could not be confirmed.";
      setStatus(statusElement, message, "warning");
    }
  }

  async function copyPassword(index: number): Promise<void> {
    if (state.copyOperation !== null || !state.batchId || !policyReady(state.clipboardStatus)) return;
    const batchId = state.batchId;
    const operation = state.operation;
    state.clipboardStatusEpoch += 1;
    const statusEpoch = state.clipboardStatusEpoch;
    stopClipboardTimer();
    state.copyOperation = operation;
    renderResults();
    setStatus(statusElement, "Copying through LocalPass\u2026");
    try {
      const parsed = parseClipboardStatus(
        await invoke<unknown>("copy_password", { batchId, rowIndex: index }),
      );
      if (!parsed) throw new Error("invalid_status");
      if (operation !== state.operation) return;
      if (batchId !== state.batchId) return;
      if (statusEpoch !== state.clipboardStatusEpoch) return;
      state.copiedIndex = index;
      state.clipboardStatus = parsed;
      state.lastSeenReleaseSequence = parsed.release_sequence;
      presentClipboardStatus(parsed);
      renderResults();
    } catch (error) {
      if (operation !== state.operation) return;
      if (statusEpoch !== state.clipboardStatusEpoch) return;
      const message = isCode(error, "not_implemented")
        ? "Password was not copied. Native clipboard handling is not available yet."
        : isCode(error, "stale")
          ? "That password batch is no longer available."
          : "Password was not copied.";
      setStatus(statusElement, message, "error");
    } finally {
      if (state.copyOperation === operation) {
        state.copyOperation = null;
        renderResults();
      }
    }
  }

  async function refreshClipboardStatus(initial: boolean): Promise<void> {
    if (state.clipboardRefreshInFlight) {
      state.clipboardRefreshQueued = true;
      return;
    }
    state.clipboardRefreshInFlight = true;
    const statusEpoch = state.clipboardStatusEpoch;
    try {
      const parsed = parseClipboardStatus(await invoke<unknown>("clipboard_status"));
      if (!parsed) throw new Error("invalid_status");
      if (statusEpoch !== state.clipboardStatusEpoch) return;
      state.clipboardStatus = parsed;
      if (initial) state.lastSeenReleaseSequence = parsed.release_sequence;
      if (!policyReady(parsed)) {
        stopClipboardTimer();
        const message = parsed.policy === "unsupported"
          ? "Clipboard operations are unsupported on this platform."
          : initial
            ? "Clipboard lifecycle is not available yet. Password generation still works."
            : "Clipboard lifecycle is unavailable. Clipboard release could not be confirmed.";
        setStatus(statusElement, message, "warning");
      } else {
        presentClipboardStatus(parsed);
      }
      renderResults();
    } catch {
      if (statusEpoch !== state.clipboardStatusEpoch) return;
      state.clipboardStatus = null;
      stopClipboardTimer();
      if (initial) setStatus(statusElement, "Clipboard status is unavailable. Copy is disabled.", "warning");
      renderResults();
    } finally {
      state.clipboardRefreshInFlight = false;
      if (state.clipboardRefreshQueued) {
        state.clipboardRefreshQueued = false;
        void refreshClipboardStatus(false);
      }
    }
  }

  function presentClipboardStatus(status: ClipboardStatus): void {
    stopClipboardTimer();
    if (status.state === "countdown" && typeof status.deadline_ms === "number") {
      updateCountdown(status.deadline_ms);
      state.clipboardTimer = window.setInterval(() => void refreshClipboardStatus(false), 1000);
      return;
    }
    if (status.state === "release_pending") {
      setStatus(statusElement, "Clipboard release pending\u2026", "neutral");
      state.clipboardTimer = window.setInterval(() => void refreshClipboardStatus(false), 1000);
      return;
    }
    const releaseAdvanced = status.release_sequence > state.lastSeenReleaseSequence;
    if (releaseAdvanced) state.lastSeenReleaseSequence = status.release_sequence;
    if (status.state === "policy_off" && state.copiedIndex !== null) {
      const number = " " + String(state.copiedIndex + 1).padStart(2, "0");
      setStatus(statusElement, "COPIED" + number + " \u00B7 AUTO-CLEAR OFF \u00B7 REMAINS UNTIL REPLACED", "warning");
      return;
    }
    const outcome = releaseAdvanced ? releaseOutcomeStatus(status.last_release) : null;
    if (outcome) {
      setStatus(statusElement, outcome.message, outcome.tone);
      state.copiedIndex = null;
      return;
    }
    if (status.state === "policy_off") {
      setStatus(statusElement, "Auto-clear is off. Clipboard content remains until replaced.", "warning");
      return;
    }
    setStatus(statusElement, "LocalPass holds no active clipboard lease.", "neutral", 3200);
    state.copiedIndex = null;
  }

  function updateCountdown(deadline: number): void {
    const seconds = Math.max(0, Math.ceil((deadline - Date.now()) / 1000));
    if (seconds === 0) {
      setStatus(statusElement, "Clipboard release pending\u2026");
      return;
    }
    const number = state.copiedIndex === null ? "" : " " + String(state.copiedIndex + 1).padStart(2, "0");
    setStatus(statusElement, "COPIED" + number + " \u00B7 RELEASE IN " + seconds + " S", "success");
  }

  function stopClipboardTimer(): void {
    if (state.clipboardTimer !== 0) {
      window.clearInterval(state.clipboardTimer);
      state.clipboardTimer = 0;
    }
  }

  async function setPinned(enabled: boolean): Promise<void> {
    pinButton.disabled = true;
    try {
      await invoke("set_always_on_top", { enabled });
      state.pinned = enabled;
      applyPressedStates();
    } catch {
      setStatus(statusElement, "Always-on-top control is not available yet.", "warning", 2600);
    } finally {
      pinButton.disabled = false;
    }
  }

  async function openAbout(): Promise<void> {
    aboutButton.disabled = true;
    try {
      await invoke("open_about", { theme: state.theme });
    } catch {
      setStatus(statusElement, "About window is not available yet.", "warning", 2600);
    } finally {
      aboutButton.disabled = false;
    }
  }

  async function requestClose(): Promise<void> {
    let epoch: number;
    try {
      redactResults();
      epoch = state.viewEpoch;
    } catch {
      setStatus(statusElement, "Close stopped because display redaction failed.", "error");
      return;
    }
    try {
      await invoke("request_close", { redactedViewEpoch: epoch });
    } catch {
      setStatus(statusElement, "Close is not available yet. Passwords remain redacted.", "warning");
    }
  }

  async function requestWindowView(next: "expanded" | "rolled", announce = true): Promise<void> {
    const previous = state.rolled ? "rolled" : "expanded";
    if (next === "expanded") applyWindowView("expanded");
    const height = measuredContentHeight();
    try {
      await invoke("set_window_view", {
        view: next,
        contentHeight: height,
        highContrast: highContrastMedia.matches,
      });
      if (next === "rolled") applyWindowView("rolled");
    } catch {
      if (next === "expanded" && previous === "rolled") applyWindowView("rolled");
      if (announce) setStatus(statusElement, "Window roll-up is not available yet.", "warning", 2400);
    }
  }

  function applyWindowView(next: "expanded" | "rolled"): void {
    state.rolled = next === "rolled";
    shell.classList.toggle("shell--rolled", state.rolled);
    expandedContent.hidden = state.rolled;
    rolledContent.hidden = !state.rolled;
  }

  function reportContentHeight(): void {
    if (state.layoutFrame !== 0) return;
    state.layoutFrame = window.requestAnimationFrame(() => {
      state.layoutFrame = 0;
      if (state.rolled) return;
      const height = measuredContentHeight();
      if (Math.abs(height - state.lastReportedHeight) < 1) return;
      state.lastReportedHeight = height;
      void invoke("set_window_view", {
        view: "expanded",
        contentHeight: height,
        highContrast: highContrastMedia.matches,
      }).catch(() => undefined);
    });
  }

  function measuredContentHeight(): number {
    return Math.max(1, Math.ceil(shell.getBoundingClientRect().height));
  }

  function scrubBeforeUnload(): void {
    layoutObserver.disconnect();
    resultsList.querySelectorAll<HTMLElement>(".password-text").forEach((element) => {
      element.textContent = "";
    });
    state.passwords.fill("");
    state.passwords = [];
    state.batchId = null;
  }
}

async function startAbout(): Promise<void> {
  const mainView = required<HTMLElement>("main-view");
  const aboutView = required<HTMLElement>("about-view");
  const shell = required<HTMLElement>("about-shell");
  const dragRegion = required<HTMLElement>("about-drag-region");
  const closeButton = required<HTMLButtonElement>("about-close-button");
  const timerValue = required<HTMLOutputElement>("timer-value");
  const timerMinus = required<HTMLButtonElement>("timer-minus");
  const timerPlus = required<HTMLButtonElement>("timer-plus");
  const timerNote = required<HTMLElement>("timer-note");
  const macSection = required<HTMLElement>("macos-policy");
  const macToggle = required<HTMLInputElement>("macos-clear-toggle");
  const macStatus = required<HTMLElement>("macos-policy-status");
  const statusElement = required<HTMLElement>("about-status");
  const mac = isMacOS();
  const state = {
    theme: queryTheme(),
    clipboardStatus: null as ClipboardStatus | null,
    timerPending: false,
    macPending: false,
  };

  mainView.hidden = true;
  aboutView.hidden = false;
  root.dataset.theme = state.theme;
  macSection.hidden = !mac;
  macToggle.disabled = true;

  closeButton.addEventListener("click", () => void closeAbout());
  dragRegion.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) return;
    if (event.target instanceof Element && event.target.closest("button, input")) return;
    void invoke("start_window_drag").catch(() => {
      setStatus(statusElement, "Window drag is not available yet.", "warning", 2400);
    });
  });
  timerMinus.addEventListener("click", () => void changeTimeout(-5));
  timerPlus.addEventListener("click", () => void changeTimeout(5));
  macToggle.addEventListener("change", () => void changeMacPolicy(macToggle.checked));
  shell.addEventListener("pointerenter", () => {
    void invoke("set_pointer_inside", { inside: true }).catch(() => undefined);
  });
  shell.addEventListener("pointerleave", () => {
    void invoke("set_pointer_inside", { inside: false }).catch(() => undefined);
  });
  window.addEventListener("beforeunload", () => {
    statusElement.textContent = "";
  });
  installShortcutGuards(statusElement);
  await refreshSettings();

  async function refreshSettings(): Promise<void> {
    try {
      const parsed = parseClipboardStatus(await invoke<unknown>("clipboard_status"));
      if (!parsed) throw new Error("invalid_status");
      state.clipboardStatus = parsed;
      renderSettings();
      if (!policyReady(parsed)) {
        setStatus(statusElement, "Native clipboard lifecycle is not available yet.", "warning");
      }
    } catch {
      state.clipboardStatus = null;
      renderSettings();
      setStatus(statusElement, "Clipboard settings are unavailable.", "warning");
    }
  }

  function renderSettings(): void {
    const clipboard = state.clipboardStatus;
    const ready = policyReady(clipboard);
    const seconds = clipboard?.timeout_seconds ?? 30;
    timerValue.value = String(seconds);
    timerValue.textContent = seconds + " S";
    const macOff = mac && !(clipboard?.macos_best_effort_clear ?? false);
    timerMinus.disabled = state.timerPending || !ready || macOff || seconds <= 5;
    timerPlus.disabled = state.timerPending || !ready || macOff || seconds >= 60;
    timerNote.textContent = !ready
      ? "Unavailable until native clipboard lifecycle is implemented."
      : macOff
        ? "Timer applies after macOS auto-clear is enabled."
        : "Applies to the active eligible clipboard lease.";
    if (mac) {
      macToggle.checked = clipboard?.macos_best_effort_clear ?? false;
      macToggle.disabled = state.macPending || !ready;
      macStatus.textContent = macToggle.checked
        ? "On for future copies only."
        : "Off for this session.";
    }
  }

  async function changeTimeout(delta: number): Promise<void> {
    const current = state.clipboardStatus?.timeout_seconds;
    if (!policyReady(state.clipboardStatus) || typeof current !== "number" || state.timerPending) return;
    const seconds = Math.max(5, Math.min(60, current + delta));
    if (seconds === current) return;
    state.timerPending = true;
    renderSettings();
    try {
      const parsed = parseClipboardStatus(
        await invoke<unknown>("set_clipboard_timeout", { seconds }),
      );
      if (!parsed) throw new Error("invalid_status");
      state.clipboardStatus = parsed;
      renderSettings();
      setStatus(statusElement, "Clipboard release timer set to " + seconds + " seconds.", "success", 2600);
    } catch {
      setStatus(statusElement, "Clipboard release timer could not be changed.", "error");
    } finally {
      state.timerPending = false;
      renderSettings();
    }
  }

  async function changeMacPolicy(enabled: boolean): Promise<void> {
    if (!mac || !policyReady(state.clipboardStatus) || state.macPending) return;
    const prior = state.clipboardStatus?.macos_best_effort_clear ?? false;
    const priorState = state.clipboardStatus?.state;
    const priorReleaseSequence = state.clipboardStatus?.release_sequence ?? 0;
    state.macPending = true;
    macToggle.disabled = true;
    setStatus(statusElement, enabled ? "Enabling for future copies\u2026" : "Disabling macOS auto-clear\u2026");
    try {
      const parsed = parseClipboardStatus(
        await invoke<unknown>("set_macos_best_effort_clear", { enabled }),
      );
      if (!parsed) throw new Error("invalid_status");
      state.clipboardStatus = parsed;
      renderSettings();
      if (enabled) {
        setStatus(statusElement, "macOS auto-clear is on for future copies only.", "warning");
      } else {
        const releaseCompleted =
          (priorState === "countdown" || priorState === "release_pending") &&
          parsed.release_sequence > priorReleaseSequence;
        const outcome = releaseCompleted ? releaseOutcomeStatus(parsed.last_release) : null;
        if (outcome) {
          setStatus(statusElement, "macOS auto-clear is off. " + outcome.message, outcome.tone);
          return;
        }
        const disarmed = parsed.policy === "best_effort_disarmed";
        const detail = disarmed
          ? "Active clipboard release disarmed. Copied content remains until replaced."
          : "No active clipboard release was armed.";
        setStatus(statusElement, "macOS auto-clear is off. " + detail, disarmed ? "warning" : "success");
      }
    } catch {
      macToggle.checked = prior;
      setStatus(
        statusElement,
        enabled
          ? "macOS auto-clear remains off. The risk opt-in was not applied."
          : "macOS auto-clear could not be disabled. Current state is unchanged.",
        "error",
      );
    } finally {
      state.macPending = false;
      renderSettings();
    }
  }

  async function closeAbout(): Promise<void> {
    closeButton.disabled = true;
    try {
      await invoke("close_about");
    } catch {
      setStatus(statusElement, "About window close is not available yet.", "warning", 2600);
    } finally {
      closeButton.disabled = false;
    }
  }
}
