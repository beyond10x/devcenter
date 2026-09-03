<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as monaco from "monaco-editor/editor/editor.api.js";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import CssWorker from "monaco-editor/language/css/css.worker.js?worker";
import HtmlWorker from "monaco-editor/language/html/html.worker.js?worker";
import JsonWorker from "monaco-editor/language/json/json.worker.js?worker";
import TypeScriptWorker from "monaco-editor/language/typescript/ts.worker.js?worker";
import {
  currentWorkbenchTheme,
  resolveWorkbenchTheme,
  WORKBENCH_MONO_FONT,
} from "./workbenchTheme";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    path: string;
    language?: string | null;
    readOnly?: boolean;
  }>(),
  { language: "plaintext", readOnly: false },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
  save: [];
  selection: [selection: { startLine: number; endLine: number; content: string } | undefined];
}>();

const host = ref<HTMLElement>();
let editor: monaco.editor.IStandaloneCodeEditor | undefined;
let contentSubscription: monaco.IDisposable | undefined;
let selectionSubscription: monaco.IDisposable | undefined;
let themeObserver: MutationObserver | undefined;

type MonacoGlobal = typeof globalThis & {
  MonacoEnvironment?: { getWorker: (_moduleId: string, label: string) => Worker };
};
(globalThis as MonacoGlobal).MonacoEnvironment = {
  getWorker: (_moduleId, label) => {
    if (label === "json") return new JsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new CssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new HtmlWorker();
    if (label === "typescript" || label === "javascript") return new TypeScriptWorker();
    return new EditorWorker();
  },
};

onMounted(async () => {
  if (!host.value) return;
  const language = normalizeLanguage(props.language, props.path);
  await Promise.all([loadLanguage(language), loadWorkbenchFont()]);
  registerEditorThemes();
  const workbenchTheme = currentWorkbenchTheme();
  editor = monaco.editor.create(host.value, {
    value: props.modelValue,
    language,
    readOnly: props.readOnly,
    automaticLayout: true,
    ariaLabel: `Editor for ${props.path}`,
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
    renderWhitespace: "selection",
    scrollBeyondLastLine: false,
    tabSize: 2,
    padding: { top: 10, bottom: 10 },
    theme: workbenchTheme.monacoName,
  });
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => emit("save"));
  contentSubscription = editor.onDidChangeModelContent(() => {
    emit("update:modelValue", editor?.getValue() ?? "");
  });
  selectionSubscription = editor.onDidChangeCursorSelection(
    ({ selection }: monaco.editor.ICursorSelectionChangedEvent) => {
      if (selection.isEmpty()) {
        emit("selection", undefined);
        return;
      }
      emit("selection", {
        startLine: selection.startLineNumber,
        endLine: selection.endLineNumber,
        content: editor?.getModel()?.getValueInRange(selection) ?? "",
      });
    },
  );
  themeObserver = new MutationObserver(() => monaco.editor.setTheme(editorTheme()));
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
});

watch(
  () => props.modelValue,
  (value) => {
    if (editor && editor.getValue() !== value) editor.setValue(value);
  },
);
watch(
  () => [props.language, props.path] as const,
  async ([language, path]) => {
    const model = editor?.getModel();
    const normalized = normalizeLanguage(language, path);
    await loadLanguage(normalized);
    if (model) monaco.editor.setModelLanguage(model, normalized);
  },
);
watch(
  () => props.readOnly,
  (readOnly) => editor?.updateOptions({ readOnly }),
);

onBeforeUnmount(() => {
  themeObserver?.disconnect();
  selectionSubscription?.dispose();
  contentSubscription?.dispose();
  editor?.dispose();
});

function editorTheme(): string {
  return currentWorkbenchTheme().monacoName;
}

function registerEditorThemes() {
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
}

async function loadWorkbenchFont() {
  await document.fonts.load(`13px ${WORKBENCH_MONO_FONT}`);
}

function normalizeLanguage(language: string | null | undefined, path: string): string {
  const declared = language?.toLocaleLowerCase();
  const extension = path.split(".").pop()?.toLocaleLowerCase();
  const candidate = declared || extension || "plaintext";
  return (
    (
      {
        bash: "shell",
        cs: "csharp",
        js: "javascript",
        jsx: "javascript",
        md: "markdown",
        py: "python",
        rs: "rust",
        sh: "shell",
        ts: "typescript",
        tsx: "typescript",
        yml: "yaml",
      } as Record<string, string>
    )[candidate] ?? candidate
  );
}

async function loadLanguage(language: string): Promise<void> {
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
    case "xml":
      await import("monaco-editor/languages/definitions/xml/register.js");
      return;
    case "ini":
      await import("monaco-editor/languages/definitions/ini/register.js");
      return;
    default:
      return;
  }
}
</script>

<template>
  <div ref="host" class="hosted-monaco-editor"></div>
</template>
