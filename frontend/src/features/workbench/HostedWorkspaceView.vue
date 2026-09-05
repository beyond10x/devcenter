<script setup lang="ts">
import { CircleAlert, LoaderCircle, ShieldCheck } from "@lucide/vue";
import { WorkbenchController, WorkbenchRefusal } from "@b10x/agentide-ui/controller";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { ApiError, errorMessage } from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";
import { DevcenterWorkbenchHost, type StartupProgress } from "./devcenterWorkbenchHost";
import {
  enterWorkspaceStartup,
  loadWorkbenchRenderer,
  markWorkspaceStage,
} from "./workspaceStartup";

type ViewState = "loading" | "ready" | "refused" | "error";
const route = useRoute();
const workspace = useWorkspaceStore();
const container = ref<HTMLElement>();
const state = ref<ViewState>("loading");
const detail = ref("");
const progress = ref<StartupProgress[]>([]);
const preparation = ref<{ message: string }>();
let generation = 0;
let host: DevcenterWorkbenchHost | undefined;
let controller: WorkbenchController | undefined;
let renderer: ReturnType<WorkbenchController["mount"]> | undefined;

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
  disposeWorkbench();
  enterWorkspaceStartup();
  state.value = "loading";
  detail.value = "";
  progress.value = [];
  preparation.value = undefined;
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
  const nextHost = new DevcenterWorkbenchHost(projectId, sessionId, workspace, (next) => {
    if (current === generation) progress.value = next;
  });
  const nextController = new WorkbenchController(nextHost);
  nextController.subscribe((frame) => {
    if (current === generation) preparation.value = frame.preparation;
  });
  // Retain pending instances immediately so navigation cancels validation and restoration too.
  host = nextHost;
  controller = nextController;
  try {
    const [module] = await Promise.all([loadWorkbenchRenderer(), nextController.start()]);
    if (current !== generation) return;
    state.value = "ready";
    await nextTick();
    if (current !== generation) return;
    if (!container.value) throw new Error("workbench_mount_absent");
    renderer = module.mountWorkbenchRenderer(nextController, container.value, async () => {
      try {
        await nextHost.revalidate();
        nextHost.retryStartup();
        await nextController.refresh();
      } catch (error) {
        showFailure(error, current);
      }
    });
    let refreshing = false;
    let refreshPending = false;
    const refresh = async () => {
      refreshPending = true;
      if (refreshing) return;
      refreshing = true;
      try {
        while (refreshPending && current === generation) {
          refreshPending = false;
          await nextController.refresh();
        }
      } catch (error) {
        showFailure(error, current);
      } finally {
        refreshing = false;
      }
    };
    nextHost.attachRenderer(renderer, () => void refresh());
    await refresh();
    if (current === generation) markWorkspaceStage("workspace-visible");
  } catch (error) {
    if (current !== generation) return;
    disposeWorkbench();
    showFailure(error, current);
  }
}

function showFailure(error: unknown, current: number): void {
  if (current !== generation) return;
  const refused =
    error instanceof WorkbenchRefusal ||
    (error instanceof ApiError && [401, 403].includes(error.status));
  disposeWorkbench();
  state.value = refused ? "refused" : "error";
  detail.value = error instanceof WorkbenchRefusal ? error.message : errorMessage(error);
}

function disposeWorkbench() {
  renderer?.destroy();
  controller?.destroy();
  host?.destroy();
  renderer = undefined;
  controller = undefined;
  host = undefined;
}
</script>

<template>
  <main v-if="state !== 'ready'" class="hosted-workbench">
    <section v-if="state === 'loading'" class="workbench-gate-state">
      <LoaderCircle class="spinning" :size="26" />
      <strong>Opening the Git workspace…</strong>
      <p>Checking access to this coding session.</p>
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
  <div v-else class="hosted-workspace-content">
    <p v-if="preparation" class="workbench-startup" role="status">{{ preparation.message }}</p>
    <div
      v-if="progress.some((part) => part.state !== 'ready')"
      class="workbench-startup"
      aria-label="Workspace loading progress"
      aria-live="polite"
    >
      <span v-for="part in progress.filter((part) => part.state !== 'ready')" :key="part.part">
        <LoaderCircle v-if="part.state === 'loading'" class="spinning" :size="13" />
        <span>{{
          part.state === "loading" ? part.message : `${part.label}: ${part.message}`
        }}</span>
        <button
          v-if="part.state === 'error' || part.state === 'refused'"
          class="button small"
          type="button"
          @click="host?.retryStartup(part.part)"
        >
          Retry {{ part.label }}
        </button>
      </span>
    </div>
    <div ref="container" class="devcenter-agentide-host" />
  </div>
</template>
