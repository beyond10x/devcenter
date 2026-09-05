<script setup lang="ts">
import { CircleAlert, LoaderCircle, ShieldCheck } from "@lucide/vue";
import { ghosttyAdapter, type GhosttyTerminal } from "@b10x/agentide-ui/adapters/terminal";
import { monacoAdapter, type MonacoApi } from "@b10x/agentide-ui/adapters/editor";
import { WorkbenchController, WorkbenchRefusal } from "@b10x/agentide-ui/controller";
import { rendererActionFormat } from "@b10x/agentide-ui/protocol";
import { createVueRenderer } from "@b10x/agentide-ui/vue";
import "@b10x/agentide-ui/styles";
import * as monaco from "monaco-editor/editor/editor.api.js";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import CssWorker from "monaco-editor/language/css/css.worker.js?worker";
import HtmlWorker from "monaco-editor/language/html/html.worker.js?worker";
import JsonWorker from "monaco-editor/language/json/json.worker.js?worker";
import TypeScriptWorker from "monaco-editor/language/typescript/ts.worker.js?worker";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { ApiError, errorMessage } from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";
import { DevcenterWorkbenchHost } from "./devcenterWorkbenchHost";
import {
  currentWorkbenchTheme,
  resolveWorkbenchTheme,
  WORKBENCH_MONO_FONT,
} from "./workbenchTheme";

type ViewState = "loading" | "ready" | "refused" | "error";
type Disposable = { dispose(): void };
type GhosttyFitAddon = { fit(): void; observeResize(): void; dispose(): void };
type NativeTerminal = GhosttyTerminal & { loadAddon(addon: GhosttyFitAddon): void };
type GhosttyModule = {
  init(): Promise<void>;
  Terminal: new (options: {
    fontFamily: string;
    fontSize: number;
    scrollback: number;
    theme: ReturnType<typeof currentWorkbenchTheme>["terminal"];
  }) => NativeTerminal;
  FitAddon: new () => GhosttyFitAddon;
};
type GhosttyWindow = Window & {
  __devcenterGhostty?: GhosttyModule;
  __devcenterGhosttyPromise?: Promise<GhosttyModule>;
};

globalThis.MonacoEnvironment = {
  getWorker(_moduleId: string, label: string) {
    if (label === "json") return new JsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new CssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new HtmlWorker();
    if (label === "typescript" || label === "javascript") return new TypeScriptWorker();
    return new EditorWorker();
  },
};

const route = useRoute();
const workspace = useWorkspaceStore();
const container = ref<HTMLElement>();
const state = ref<ViewState>("loading");
const detail = ref("");
let generation = 0;
let host: DevcenterWorkbenchHost | undefined;
let controller: WorkbenchController | undefined;
let renderer: ReturnType<WorkbenchController["mount"]> | undefined;
let themeObserver: MutationObserver | undefined;

onMounted(mountWorkbench);
watch(
  () => [route.params.projectId, route.params.sessionId],
  () => void mountWorkbench(),
);
onBeforeUnmount(() => {
  generation += 1;
  disposeWorkbench();
});

async function mountWorkbench() {
  const current = ++generation;
  let nextHost: DevcenterWorkbenchHost | undefined;
  let nextController: WorkbenchController | undefined;
  let nextRenderer: ReturnType<WorkbenchController["mount"]> | undefined;
  disposeWorkbench();
  state.value = "loading";
  detail.value = "";
  if (!workspace.session?.agentide_workspace_enabled) {
    state.value = "refused";
    detail.value = "This deployment has not enabled the hosted AgentIDE workbench.";
    return;
  }
  const projectId = String(route.params.projectId ?? "");
  const sessionId = String(route.params.sessionId ?? "");
  if (!projectId || !sessionId) {
    state.value = "refused";
    detail.value = "The project and coding-session route are required.";
    return;
  }
  try {
    nextHost = new DevcenterWorkbenchHost(projectId, sessionId, workspace);
    nextController = new WorkbenchController(nextHost);
    await nextController.start();
    if (current !== generation) {
      nextController.destroy();
      nextHost.destroy();
      return;
    }
    state.value = "ready";
    await nextTick();
    if (!container.value || current !== generation) throw new Error("workbench_mount_absent");
    const target = createVueRenderer({
      editor: monacoAdapter(monacoPort()),
      terminal: ghosttyAdapter(() => new DeferredGhosttyTerminal()),
    });
    installMonacoThemes();
    themeObserver = new MutationObserver(() => {
      monaco.editor.setTheme(currentWorkbenchTheme().monacoName);
    });
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    const mountedController = nextController;
    nextRenderer = mountedController.mount(target, container.value);
    nextHost.attachRenderer(
      nextRenderer,
      () => void mountedController.dispatch({ format: rendererActionFormat, kind: "refresh" }),
    );
    host = nextHost;
    controller = nextController;
    renderer = nextRenderer;
    void mountedController.dispatch({
      format: rendererActionFormat,
      kind: "load_tree",
      path: "",
    });
  } catch (caught) {
    nextRenderer?.destroy();
    nextController?.destroy();
    nextHost?.destroy();
    themeObserver?.disconnect();
    themeObserver = undefined;
    if (current !== generation) return;
    state.value =
      caught instanceof WorkbenchRefusal || (caught instanceof ApiError && caught.status === 403)
        ? "refused"
        : "error";
    detail.value = caught instanceof WorkbenchRefusal ? caught.message : errorMessage(caught);
  }
}

function disposeWorkbench() {
  themeObserver?.disconnect();
  themeObserver = undefined;
  renderer?.destroy();
  controller?.destroy();
  host?.destroy();
  renderer = undefined;
  controller = undefined;
  host = undefined;
}

function monacoPort(): MonacoApi {
  const uris = new WeakMap<object, monaco.Uri>();
  const models = new WeakMap<object, monaco.editor.ITextModel>();
  return {
    Uri: {
      parse(value) {
        const reference = {};
        uris.set(reference, monaco.Uri.parse(value));
        return reference;
      },
    },
    editor: {
      createModel(value, language, uri) {
        const registered = monaco.languages
          .getLanguages()
          .some((candidate) => candidate.id === language);
        const native = monaco.editor.createModel(
          value,
          registered ? language : "plaintext",
          uris.get(uri),
        );
        void loadMonacoLanguage(language).then(() => {
          if (!native.isDisposed()) monaco.editor.setModelLanguage(native, language);
        });
        const model = {
          dispose: () => native.dispose(),
          getValue: () => native.getValue(),
          setValue: (next: string) => native.setValue(next),
        };
        models.set(model, native);
        return model;
      },
      create(element, options) {
        const model = models.get(options.model);
        if (!model) throw new Error("agentide_monaco_model_absent");
        const editor = monaco.editor.create(element, {
          ...options,
          model,
          ariaLabel: "Workspace source editor",
          bracketPairColorization: { enabled: true },
          folding: true,
          fontFamily: WORKBENCH_MONO_FONT,
          fontLigatures: true,
          fontSize: 13,
          fontWeight: "450",
          lineHeight: 20,
          lineNumbers: "on",
          lineNumbersMinChars: 3,
          minimap: { enabled: false },
          multiCursorModifier: "alt",
          padding: { top: 10, bottom: 10 },
          renderWhitespace: "selection",
          scrollBeyondLastLine: false,
          tabSize: 2,
          theme: currentWorkbenchTheme().monacoName,
        });
        return {
          dispose: () => editor.dispose(),
          focus: () => editor.focus(),
          updateOptions: (next) => editor.updateOptions(next),
          onDidChangeModelContent: (listener) => editor.onDidChangeModelContent(listener),
        };
      },
      setModelLanguage(model, language) {
        const native = models.get(model);
        if (!native) throw new Error("agentide_monaco_model_absent");
        void loadMonacoLanguage(language).then(() => {
          if (!native.isDisposed()) monaco.editor.setModelLanguage(native, language);
        });
      },
    },
  };
}

class DeferredGhosttyTerminal implements GhosttyTerminal {
  readonly #data = new Set<(data: string) => void>();
  readonly #resize = new Set<(size: { cols: number; rows: number }) => void>();
  readonly #queued: Uint8Array[] = [];
  #native?: NativeTerminal;
  #fit?: GhosttyFitAddon;
  #dataSubscription?: Disposable;
  #resizeSubscription?: Disposable;
  #container?: HTMLElement;
  #columns = 100;
  #rows = 28;
  #disposed = false;

  open(container: HTMLElement): void {
    this.#container = container;
    void this.#mount().catch(() => {
      if (!this.#disposed && this.#container) {
        this.#container.textContent = "The terminal renderer could not be loaded.";
      }
    });
  }

  write(bytes: Uint8Array): void {
    if (this.#native) this.#native.write(bytes);
    else this.#queued.push(bytes.slice());
  }

  focus(): void {
    this.#native?.focus();
  }

  resize(columns: number, rows: number): void {
    this.#columns = columns;
    this.#rows = rows;
    this.#native?.resize(columns, rows);
  }

  dispose(): void {
    this.#disposed = true;
    this.#dataSubscription?.dispose();
    this.#resizeSubscription?.dispose();
    this.#fit?.dispose();
    this.#native?.dispose();
    this.#data.clear();
    this.#resize.clear();
    this.#queued.length = 0;
  }

  onData(listener: (data: string) => void): Disposable {
    this.#data.add(listener);
    return { dispose: () => this.#data.delete(listener) };
  }

  onResize(listener: (size: { cols: number; rows: number }) => void): Disposable {
    this.#resize.add(listener);
    return { dispose: () => this.#resize.delete(listener) };
  }

  async #mount(): Promise<void> {
    const module = await loadGhostty();
    await module.init();
    await document.fonts.load(`13px ${WORKBENCH_MONO_FONT}`);
    if (this.#disposed || !this.#container) return;
    const native = new module.Terminal({
      fontFamily: WORKBENCH_MONO_FONT,
      fontSize: 13,
      scrollback: 10_000,
      theme: currentWorkbenchTheme().terminal,
    });
    const fit = new module.FitAddon();
    native.loadAddon(fit);
    native.open(this.#container);
    native.resize(this.#columns, this.#rows);
    this.#dataSubscription = native.onData((data) => {
      for (const listener of this.#data) listener(data);
    });
    this.#resizeSubscription = native.onResize((size) => {
      for (const listener of this.#resize) listener(size);
    });
    this.#native = native;
    this.#fit = fit;
    fit.fit();
    fit.observeResize();
    for (const bytes of this.#queued) native.write(bytes);
    this.#queued.length = 0;
    native.focus();
  }
}

function installMonacoThemes() {
  for (const id of ["light", "dark", "monokai", "solarized-light", "solarized-dark"] as const) {
    const theme = resolveWorkbenchTheme(id);
    monaco.editor.defineTheme(theme.monacoName, {
      base: theme.monacoBase,
      inherit: true,
      rules: [
        { token: "keyword", foreground: theme.syntax.keyword.slice(1), fontStyle: "bold" },
        { token: "keyword.type", foreground: theme.syntax.type.slice(1), fontStyle: "bold" },
        { token: "string", foreground: theme.syntax.string.slice(1) },
        { token: "string.quote", foreground: theme.syntax.string.slice(1) },
        { token: "string.escape", foreground: theme.syntax.operator.slice(1), fontStyle: "bold" },
        { token: "number", foreground: theme.syntax.number.slice(1) },
        { token: "comment", foreground: theme.syntax.comment.slice(1), fontStyle: "italic" },
        { token: "operator", foreground: theme.syntax.operator.slice(1) },
      ],
      colors: {
        "editor.background": theme.editorBackground,
        "editor.foreground": theme.editorForeground,
        "editor.lineHighlightBackground": theme.editorLineHighlight,
        "editor.selectionBackground": theme.editorSelection,
        "editor.inactiveSelectionBackground": theme.editorInactiveSelection,
        "editorCursor.foreground": theme.editorCursor,
        "editorLineNumber.foreground": theme.editorLineNumber,
        "editorLineNumber.activeForeground": theme.editorActiveLineNumber,
        "editorIndentGuide.background1": theme.editorLineHighlight,
        "editorIndentGuide.activeBackground1": theme.editorLineNumber,
      },
    });
  }
  monaco.editor.setTheme(currentWorkbenchTheme().monacoName);
}

async function loadMonacoLanguage(language: string): Promise<void> {
  switch (language) {
    case "typescript":
    case "javascript":
      await import("monaco-editor/language/typescript/monaco.contribution.js");
      return;
    case "json":
      await import("monaco-editor/language/json/monaco.contribution.js");
      return;
    case "css":
    case "scss":
    case "less":
      await import("monaco-editor/language/css/monaco.contribution.js");
      return;
    case "html":
    case "handlebars":
    case "razor":
      await import("monaco-editor/language/html/monaco.contribution.js");
      return;
    case "rust":
      await import("monaco-editor/languages/definitions/rust/register.js");
      return;
    case "python":
      await import("monaco-editor/languages/definitions/python/register.js");
      return;
    case "shell":
      await import("monaco-editor/languages/definitions/shell/register.js");
      return;
    case "yaml":
      await import("monaco-editor/languages/definitions/yaml/register.js");
      return;
    case "go":
      await import("monaco-editor/languages/definitions/go/register.js");
      return;
    case "cpp":
      await import("monaco-editor/languages/definitions/cpp/register.js");
      return;
    case "java":
      await import("monaco-editor/languages/definitions/java/register.js");
      return;
    case "markdown":
      await import("monaco-editor/languages/definitions/markdown/register.js");
      return;
    case "sql":
      await import("monaco-editor/languages/definitions/sql/register.js");
      return;
  }
}

function loadGhostty(): Promise<GhosttyModule> {
  const target = window as GhosttyWindow;
  if (target.__devcenterGhostty) return Promise.resolve(target.__devcenterGhostty);
  if (target.__devcenterGhosttyPromise) return target.__devcenterGhosttyPromise;
  target.__devcenterGhosttyPromise = new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.type = "module";
    script.src = "/vendor/ghostty-web/loader.js";
    script.addEventListener("load", () => {
      if (target.__devcenterGhostty) resolve(target.__devcenterGhostty);
      else reject(new Error("terminal_renderer_module_invalid"));
    });
    script.addEventListener("error", () => reject(new Error("terminal_renderer_unavailable")));
    document.head.append(script);
  });
  return target.__devcenterGhosttyPromise;
}
</script>

<template>
  <main v-if="state !== 'ready'" class="hosted-workbench">
    <section v-if="state === 'loading'" class="workbench-gate-state">
      <LoaderCircle class="spinning" :size="26" />
      <strong>Opening the Git workspace…</strong>
      <p>Devcenter is revalidating this coding session and its current authority.</p>
    </section>
    <section v-else-if="state === 'refused'" class="workbench-gate-state refused-state">
      <ShieldCheck :size="28" />
      <strong>Hosted coding workspace unavailable</strong>
      <p>{{ detail }}</p>
      <RouterLink class="button" :to="`/projects/${String(route.params.projectId ?? '')}`">
        Return to project
      </RouterLink>
    </section>
    <section v-else class="workbench-gate-state error-state">
      <CircleAlert :size="28" />
      <strong>Workspace could not be opened</strong>
      <p>{{ detail }}</p>
      <button class="button" type="button" @click="mountWorkbench">Try again</button>
    </section>
  </main>
  <div v-else ref="container" class="devcenter-agentide-host" />
</template>
