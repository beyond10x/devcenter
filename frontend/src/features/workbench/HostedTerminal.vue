<script setup lang="ts">
import { Copy, Link, LoaderCircle, RefreshCw, Search, Unplug, XCircle } from "@lucide/vue";
import { onBeforeUnmount, onMounted, ref } from "vue";
import { api, ApiError, type TerminalSession } from "@/api/client";
import { currentWorkbenchTheme, type TerminalTheme, WORKBENCH_MONO_FONT } from "./workbenchTheme";

interface Disposable {
  dispose(): void;
}

interface GhosttyCell {
  codepoint: number;
}

interface GhosttyTerminal {
  cols: number;
  rows: number;
  open(host: HTMLElement): void;
  loadAddon(addon: GhosttyFitAddon): void;
  write(bytes: Uint8Array): void;
  focus(): void;
  reset(): void;
  dispose(): void;
  onData(listener: (data: string) => void): Disposable;
  onResize(listener: (size: { cols: number; rows: number }) => void): Disposable;
  getSelection(): string;
  getScrollbackLength(): number;
  getScrollbackLine(line: number): GhosttyCell[] | null;
  scrollToLine(line: number): void;
  paste(content: string): void;
}

interface GhosttyFitAddon {
  fit(): void;
  observeResize(): void;
  dispose(): void;
}

interface GhosttyModule {
  init(): Promise<void>;
  Terminal: new (options: {
    fontFamily: string;
    fontSize: number;
    scrollback: number;
    theme: TerminalTheme;
  }) => GhosttyTerminal;
  FitAddon: new () => GhosttyFitAddon;
}

interface GhosttyWindow extends Window {
  __devcenterGhostty?: GhosttyModule;
  __devcenterGhosttyPromise?: Promise<GhosttyModule>;
}

const props = defineProps<{ terminal: TerminalSession }>();
const emit = defineEmits<{
  kill: [];
  attach: [selection: { content: string; terminalId: string }];
  lifecycle: [];
}>();

const host = ref<HTMLElement>();
const connectionState = ref<
  "loading" | "connecting" | "running" | "partial" | "refused" | "exited" | "detached"
>("loading");
const detail = ref("");
const search = ref("");
let renderer: GhosttyTerminal | undefined;
let fit: GhosttyFitAddon | undefined;
let socket: WebSocket | undefined;
let reconnectTimer: number | undefined;
let reconnectAttempt = 0;
let connectionEpoch = 0;
let alive = true;
let hasSequence = false;
let lastSequence = 0n;

onMounted(open);
onBeforeUnmount(dispose);

async function open() {
  try {
    // The renderer is vendored and served by Devcenter; it is loaded only when a live terminal
    // pane exists, and its WASM is never fetched from a CDN.
    const ghostty = await loadGhostty();
    await ghostty.init();
    await document.fonts.load(`13px ${WORKBENCH_MONO_FONT}`);
    if (!alive || !host.value) return;
    renderer = new ghostty.Terminal({
      fontFamily: WORKBENCH_MONO_FONT,
      fontSize: 13,
      scrollback: 10_000,
      theme: currentWorkbenchTheme().terminal,
    });
    fit = new ghostty.FitAddon();
    renderer.loadAddon(fit);
    renderer.open(host.value);
    fit.fit();
    fit.observeResize();
    const encoder = new TextEncoder();
    renderer.onData((data) => {
      if (socket?.readyState === WebSocket.OPEN) socket.send(encoder.encode(data));
    });
    renderer.onResize(({ cols, rows }) => sendResize(cols, rows));
    void connect();
  } catch (error) {
    connectionState.value = "refused";
    detail.value = error instanceof Error ? error.message : "terminal_renderer_unavailable";
  }
}

function loadGhostty(): Promise<GhosttyModule> {
  const ghosttyWindow = window as GhosttyWindow;
  if (ghosttyWindow.__devcenterGhostty) {
    return Promise.resolve(ghosttyWindow.__devcenterGhostty);
  }
  if (ghosttyWindow.__devcenterGhosttyPromise) return ghosttyWindow.__devcenterGhosttyPromise;
  ghosttyWindow.__devcenterGhosttyPromise = new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.type = "module";
    script.src = "/vendor/ghostty-web/loader.js";
    script.addEventListener("load", () => {
      if (ghosttyWindow.__devcenterGhostty) resolve(ghosttyWindow.__devcenterGhostty);
      else reject(new Error("terminal_renderer_module_invalid"));
    });
    script.addEventListener("error", () => reject(new Error("terminal_renderer_unavailable")));
    document.head.append(script);
  });
  return ghosttyWindow.__devcenterGhosttyPromise;
}

async function connect() {
  if (!alive || !renderer) return;
  const epoch = ++connectionEpoch;
  connectionState.value = "connecting";
  detail.value = reconnectAttempt ? `Reconnect attempt ${String(reconnectAttempt)}` : "";
  try {
    const terminal = await api.terminal(props.terminal.id);
    if (connectionCancelled(epoch)) return;
    if (terminal.state !== "running" && terminal.state !== "preparing") {
      connectionState.value =
        terminal.state === "exited" || terminal.state === "terminated" ? "exited" : "refused";
      detail.value = terminal.failure_code ?? terminalExitDetail(terminal.exit);
      emit("lifecycle");
      return;
    }
  } catch (error) {
    if (connectionCancelled(epoch)) return;
    if (
      error instanceof ApiError &&
      (error.status === 404 || error.code === "workspace_terminal_not_found")
    ) {
      connectionState.value = "refused";
      detail.value = "This terminal no longer exists. Refreshing the terminal inventory.";
      emit("lifecycle");
      return;
    }
    connectionState.value = "detached";
    detail.value = "DevCenter could not verify the terminal session.";
    scheduleReconnect();
    return;
  }
  const candidate = new WebSocket(
    api.terminalSocketUrl(props.terminal.id, hasSequence ? lastSequence : undefined),
  );
  candidate.binaryType = "arraybuffer";
  socket = candidate;
  candidate.onopen = () => {
    if (socket === candidate) sendResize(renderer?.cols ?? 80, renderer?.rows ?? 24);
  };
  candidate.onmessage = (event: MessageEvent<unknown>) => {
    if (socket === candidate) receive(event);
  };
  candidate.onerror = () => {
    if (socket === candidate) detail.value = "The same-origin terminal transport is unavailable.";
  };
  candidate.onclose = () => {
    if (socket !== candidate) return;
    socket = undefined;
    if (!alive || connectionState.value === "exited" || connectionState.value === "refused") return;
    connectionState.value = "detached";
    scheduleReconnect();
  };
}

function connectionCancelled(epoch: number) {
  return !alive || epoch !== connectionEpoch;
}

function receive(event: MessageEvent<unknown>) {
  if (typeof event.data === "string") {
    receiveLifecycle(event.data);
    return;
  }
  if (!(event.data instanceof ArrayBuffer) || event.data.byteLength < 8 || !renderer) {
    connectionState.value = "refused";
    detail.value = "terminal_output_frame_invalid";
    socket?.close();
    return;
  }
  const view = new DataView(event.data);
  const sequence = view.getBigUint64(0, false);
  if (hasSequence && sequence <= lastSequence) return;
  if (hasSequence && sequence > lastSequence + 1n) {
    connectionState.value = "partial";
    detail.value = "Earlier terminal output is outside the bounded replay window.";
  }
  hasSequence = true;
  lastSequence = sequence;
  renderer.write(new Uint8Array(event.data, 8));
}

function receiveLifecycle(payload: string) {
  let lifecycle: Record<string, unknown>;
  try {
    lifecycle = JSON.parse(payload) as Record<string, unknown>;
  } catch {
    connectionState.value = "refused";
    detail.value = "terminal_lifecycle_frame_invalid";
    return;
  }
  if (lifecycle.kind === "attached") {
    const replay = lifecycle.replay as
      { complete?: unknown; newest_sequence?: unknown } | undefined;
    const newest = replay?.newest_sequence;
    const sequenceRestarted =
      hasSequence &&
      (newest === null ||
        (typeof newest === "number" &&
          Number.isSafeInteger(newest) &&
          BigInt(newest) < lastSequence));
    const partial = replay?.complete === false || sequenceRestarted;
    reconnectAttempt = 0;
    connectionState.value = partial ? "partial" : "running";
    detail.value = sequenceRestarted
      ? "The terminal output sequence restarted. Reload the output Workspace still retains."
      : replay?.complete === false
        ? "Earlier output is outside the 4 MiB replay window."
        : "";
    renderer?.focus();
    return;
  }
  if (lifecycle.kind === "exit") {
    connectionState.value = "exited";
    detail.value = terminalExitDetail(lifecycle.exit);
    emit("lifecycle");
    return;
  }
  if (lifecycle.kind === "refused") {
    connectionState.value = "refused";
    detail.value = typeof lifecycle.code === "string" ? lifecycle.code : "terminal_refused";
    emit("lifecycle");
    return;
  }
  if (lifecycle.kind === "detached") {
    connectionState.value = "detached";
    detail.value = typeof lifecycle.code === "string" ? lifecycle.code : "terminal_detached";
  }
}

function terminalExitDetail(value: unknown): string {
  if (!value || typeof value !== "object") return "Process exited.";
  const exit = value as { code?: unknown; signal?: unknown };
  if (typeof exit.signal === "string") return `Process ended by ${exit.signal}.`;
  if (typeof exit.code === "number") return `Process exited with ${String(exit.code)}.`;
  return "Process exited.";
}

function scheduleReconnect() {
  if (!alive || reconnectTimer !== undefined) return;
  reconnectAttempt += 1;
  const delay = Math.min(8_000, 500 * 2 ** Math.min(reconnectAttempt - 1, 4));
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = undefined;
    void connect();
  }, delay);
}

function replayRetainedOutput() {
  if (reconnectTimer !== undefined) {
    window.clearTimeout(reconnectTimer);
    reconnectTimer = undefined;
  }
  const previous = socket;
  socket = undefined;
  previous?.close();
  renderer?.reset();
  hasSequence = false;
  lastSequence = 0n;
  reconnectAttempt = 0;
  connectionState.value = "connecting";
  detail.value = "Reloading the oldest terminal output still retained by Workspace.";
  void connect();
}

function sendResize(columns: number, rows: number) {
  if (socket?.readyState !== WebSocket.OPEN) return;
  socket.send(JSON.stringify({ kind: "resize", columns, rows }));
}

async function copySelection() {
  const selected = renderer?.getSelection() ?? "";
  if (selected) await navigator.clipboard.writeText(selected);
}

async function pasteClipboard() {
  const content = await navigator.clipboard.readText();
  if (content) renderer?.paste(content);
}

function attachSelection() {
  const content = renderer?.getSelection() ?? "";
  if (content) emit("attach", { content, terminalId: props.terminal.id });
}

function findNext() {
  const needle = search.value.trim().toLocaleLowerCase();
  if (!needle || !renderer) return;
  const length = renderer.getScrollbackLength();
  for (let line = length - 1; line >= 0; line -= 1) {
    const cells = renderer.getScrollbackLine(line);
    const content = cells
      ?.map((cell) =>
        cell.codepoint === 0 || cell.codepoint < 32 ? " " : String.fromCodePoint(cell.codepoint),
      )
      .join("");
    if (content?.toLocaleLowerCase().includes(needle)) {
      renderer.scrollToLine(line);
      return;
    }
  }
  detail.value = `No scrollback match for “${search.value.trim()}”.`;
}

function dispose() {
  alive = false;
  connectionEpoch += 1;
  if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
  socket?.close();
  socket = undefined;
  fit?.dispose();
  fit = undefined;
  renderer?.dispose();
  renderer = undefined;
}
</script>

<template>
  <section class="hosted-terminal-screen" :aria-label="`Terminal ${terminal.id}`">
    <header class="terminal-tools">
      <span class="terminal-connection" :class="connectionState">
        <LoaderCircle
          v-if="connectionState === 'loading' || connectionState === 'connecting'"
          class="spinning"
          :size="12"
        />
        {{ connectionState }}
      </span>
      <label>
        <Search :size="12" />
        <input v-model="search" aria-label="Search terminal scrollback" @keydown.enter="findNext" />
      </label>
      <button type="button" title="Copy selection" @click="copySelection">
        <Copy :size="12" /> Copy
      </button>
      <button type="button" title="Paste from clipboard" @click="pasteClipboard">Paste</button>
      <button type="button" title="Attach selection to session context" @click="attachSelection">
        <Link :size="12" /> Attach
      </button>
      <button
        v-if="connectionState === 'partial'"
        type="button"
        title="Clear the terminal and reload all output still retained by Workspace"
        @click="replayRetainedOutput"
      >
        <RefreshCw :size="12" /> Reload retained output
      </button>
      <button
        class="terminal-kill"
        type="button"
        title="Kill terminal process"
        @click="emit('kill')"
      >
        <XCircle :size="12" /> Kill
      </button>
    </header>
    <p v-if="detail" class="terminal-detail" role="status">{{ detail }}</p>
    <div ref="host" class="ghostty-host" tabindex="0"></div>
    <footer>
      <span><Unplug :size="11" /> Closing this tab detaches; it does not kill the process.</span>
      <code>{{ terminal.profile.working_directory }}</code>
    </footer>
  </section>
</template>
