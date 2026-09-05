/// <reference types="vite/client" />

// Where the engineer's own `phone-server` listens. Declared rather than read as `any`, so a typo in
// the variable's name is a type error and not a phone that silently dials nowhere.
interface ImportMetaEnv {
  readonly VITE_PHONE_ENDPOINT?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare module "monaco-editor/language/*/monaco.contribution.js";

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<Record<string, never>, Record<string, never>, unknown>;
  export default component;
}
