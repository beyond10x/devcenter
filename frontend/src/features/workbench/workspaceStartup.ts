type RendererModule = typeof import("./workbenchRenderer");
let rendererModule: Promise<RendererModule> | undefined;
const marked = new Set<string>();
let pendingClick = false;

/** Fetches only static code, never a session, grant, or repository. */
export function loadWorkbenchRenderer(): Promise<RendererModule> {
  rendererModule ??= import("./workbenchRenderer").catch((error: unknown) => {
    rendererModule = undefined;
    throw error;
  });
  return rendererModule;
}

export function preloadWorkbench(): void {
  void loadWorkbenchRenderer().catch(() => undefined);
}

function clearWorkspaceMarks(): void {
  marked.clear();
  for (const entry of performance.getEntriesByType("mark")) {
    if (entry.name.startsWith("devcenter.workspace.")) performance.clearMarks(entry.name);
  }
}

export function beginWorkspaceStartup(): void {
  clearWorkspaceMarks();
  pendingClick = true;
  markWorkspaceStage("click");
}

export function enterWorkspaceStartup(): void {
  if (!pendingClick) clearWorkspaceMarks();
  pendingClick = false;
  markWorkspaceStage("route-entry");
}

/** Stable stage names intentionally contain no project, actor, or session identifiers. */
export function markWorkspaceStage(stage: string): void {
  if (marked.has(stage)) return;
  marked.add(stage);
  if (typeof performance.mark === "function") performance.mark(`devcenter.workspace.${stage}`);
}
