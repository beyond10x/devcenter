<script setup lang="ts">
import {
  ArrowUpRight,
  Bot,
  Boxes,
  Braces,
  ChevronRight,
  FileCode2,
  FolderGit2,
  GitBranch,
  LoaderCircle,
  MessageSquareText,
  Play,
  RefreshCw,
  Search,
  ShieldCheck,
} from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  api,
  errorMessage,
  type Branch,
  type EngineeringArtifact,
  type Project,
  type ProjectMessage,
  type ProjectThread,
  type RepositoryCandidate,
  type RepositoryEntry,
  type WorkflowDefinition,
  type WorkflowRun,
} from "@/api/client";

type Tab = "overview" | "files" | "chat" | "workflows" | "aep";

const route = useRoute();
const router = useRouter();
const loading = ref(true);
const error = ref("");
const search = ref("");
const repositories = ref<RepositoryCandidate[]>([]);
const project = ref<Project>();
const branches = ref<Branch[]>([]);
const repositoryTree = ref<RepositoryEntry[]>([]);
const artifacts = ref<EngineeringArtifact[]>([]);
const artifactsHaveMore = ref(false);
const aepError = ref("");
const threads = ref<ProjectThread[]>([]);
const selectedThreadId = ref<string>();
const messages = ref<ProjectMessage[]>([]);
const workflows = ref<WorkflowDefinition[]>([]);
const runs = ref<WorkflowRun[]>([]);
const activeTab = ref<Tab>("overview");
const tabs: Tab[] = ["overview", "files", "chat", "workflows", "aep"];
const opening = ref<string>();
const refreshing = ref(false);
const sending = ref(false);
const draft = ref("");
const runningWorkflow = ref<string>();

const projectId = computed(() =>
  typeof route.params.projectId === "string" ? route.params.projectId : undefined,
);
const filteredRepositories = computed(() => {
  const query = search.value.trim().toLowerCase();
  if (!query) return repositories.value;
  return repositories.value.filter((repository) =>
    repository.path_with_namespace.toLowerCase().includes(query),
  );
});
const selectedThread = computed(() =>
  threads.value.find((thread) => thread.id === selectedThreadId.value),
);

onMounted(load);
watch(projectId, load);
watch(selectedThreadId, () => void loadMessages());

async function load() {
  loading.value = true;
  error.value = "";
  try {
    if (!projectId.value) {
      repositories.value = await api.repositories();
      project.value = undefined;
    } else {
      const loadedProject = await api.project(projectId.value);
      const [loadedBranches, loadedTree, loadedThreads, loadedWorkflows] = await Promise.all([
        api.branches(projectId.value),
        api.repositoryTree(projectId.value),
        api.threads(projectId.value),
        api.workflows(projectId.value),
      ]);
      project.value = loadedProject;
      branches.value = loadedBranches;
      repositoryTree.value = loadedTree;
      threads.value = loadedThreads;
      workflows.value = loadedWorkflows;
      selectedThreadId.value = loadedThreads[0]?.id;
      if (selectedThreadId.value) await loadMessages();
      await loadEngineeringArtifacts(loadedProject.id);
    }
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
}

async function loadEngineeringArtifacts(id: string) {
  aepError.value = "";
  try {
    const page = await api.engineeringArtifacts(id);
    artifacts.value = page.artifacts;
    artifactsHaveMore.value = page.has_more;
  } catch (caught) {
    artifacts.value = [];
    artifactsHaveMore.value = false;
    aepError.value = errorMessage(caught);
  }
}

async function openRepository(repository: RepositoryCandidate) {
  opening.value = repository.project_ref;
  error.value = "";
  try {
    const opened = repository.opened_project_id
      ? await api.project(repository.opened_project_id)
      : await api.openProject(repository);
    await router.push({ name: "project", params: { projectId: opened.id } });
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    opening.value = undefined;
  }
}

async function pinBranch(branch: string) {
  if (!project.value) return;
  refreshing.value = true;
  error.value = "";
  try {
    project.value = await api.selectBranch(project.value.id, branch);
    threads.value = await api.threads(project.value.id);
    if (selectedThreadId.value) await loadMessages();
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    refreshing.value = false;
  }
}

async function ensureThread(): Promise<ProjectThread | undefined> {
  if (selectedThread.value) return selectedThread.value;
  const snapshot = project.value;
  if (!snapshot?.pinned_commit) return undefined;
  const created = await api.createThread(
    snapshot.id,
    snapshot.selected_branch,
    snapshot.pinned_commit,
    "Project agent",
  );
  threads.value = [created, ...threads.value];
  selectedThreadId.value = created.id;
  return created;
}

async function loadMessages() {
  messages.value = selectedThreadId.value ? await api.messages(selectedThreadId.value) : [];
}

async function sendMessage() {
  const content = draft.value.trim();
  if (!content) return;
  sending.value = true;
  error.value = "";
  try {
    const thread = await ensureThread();
    if (!thread) throw new Error("snapshot_missing");
    const message = await api.createMessage(thread.id, content);
    messages.value.push(message);
    draft.value = "";
    void observeAgentReply(thread.id, message.sequence);
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    sending.value = false;
  }
}

async function observeAgentReply(threadId: string, afterSequence: number) {
  try {
    for (let attempt = 0; attempt < 120; attempt += 1) {
      await new Promise((resolve) => window.setTimeout(resolve, 1000));
      const observed = await api.messages(threadId);
      messages.value = observed;
      if (observed.some((message) => message.sequence > afterSequence)) return;
    }
  } catch {
    // A later thread selection or refresh provides a natural retry boundary.
  }
}

async function startWorkflow(definition: WorkflowDefinition) {
  const snapshot = project.value;
  if (!snapshot?.pinned_commit) return;
  runningWorkflow.value = definition.id;
  error.value = "";
  try {
    const run = await api.startWorkflow(
      snapshot.id,
      definition.id,
      snapshot.selected_branch,
      snapshot.pinned_commit,
    );
    runs.value = [run, ...runs.value];
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    runningWorkflow.value = undefined;
  }
}

function shortCommit(commit?: string | null) {
  return commit?.slice(0, 10) ?? "not pinned";
}
</script>

<template>
  <div class="view projects-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Repository workspaces</p>
        <h1>{{ project ? project.path_with_namespace : "Open a project" }}</h1>
        <p v-if="project">
          Shared project, personal threads, and evidence pinned to one exact repository snapshot.
        </p>
        <p v-else>Every repository currently visible through your connected GitLab identity.</p>
      </div>
      <RouterLink v-if="project" class="button quiet" to="/projects">
        <FolderGit2 :size="17" /> Change project
      </RouterLink>
    </header>

    <div v-if="error" class="inline-state error-state" role="alert">
      <ShieldCheck :size="20" />
      <div>
        <strong>Request refused</strong>
        <p>{{ error }}</p>
      </div>
    </div>
    <div v-if="loading" class="inline-state">
      <LoaderCircle class="spinning" :size="22" /> Loading current authority…
    </div>

    <template v-else-if="!project">
      <label class="project-search">
        <Search :size="18" /><span class="sr-only">Search repositories</span>
        <input v-model="search" type="search" placeholder="Search namespace or repository" />
        <span>{{ filteredRepositories.length }} visible</span>
      </label>
      <section class="repository-list" aria-label="Visible repositories">
        <button
          v-for="repository in filteredRepositories"
          :key="`${repository.forge_instance_ref}:${repository.project_ref}`"
          type="button"
          class="repository-row"
          :disabled="opening === repository.project_ref"
          @click="openRepository(repository)"
        >
          <span class="repo-icon"><FolderGit2 :size="19" /></span>
          <span
            ><strong>{{ repository.path_with_namespace }}</strong
            ><small
              >{{ repository.visibility }} · default
              {{ repository.default_branch || "not declared" }}</small
            ></span
          >
          <span v-if="repository.opened_project_id" class="status-pill">Opened</span>
          <LoaderCircle v-if="opening === repository.project_ref" class="spinning" :size="18" />
          <ChevronRight v-else :size="18" />
        </button>
        <div v-if="!filteredRepositories.length" class="empty-projects">
          <FolderGit2 :size="28" /><strong>No matching repository</strong>
          <p>Try another name or verify the current GitLab connection.</p>
        </div>
      </section>
    </template>

    <template v-else>
      <section class="project-toolbar">
        <label
          ><GitBranch :size="16" /><span class="sr-only">Branch</span>
          <select
            :value="project.selected_branch"
            :disabled="refreshing"
            @change="pinBranch(($event.target as HTMLSelectElement).value)"
          >
            <option v-for="branch in branches" :key="branch.name" :value="branch.name">
              {{ branch.name }}
            </option>
          </select>
        </label>
        <span class="commit-chip"
          ><Braces :size="15" /> {{ shortCommit(project.pinned_commit) }}</span
        >
        <button
          class="button small"
          type="button"
          :disabled="refreshing"
          @click="pinBranch(project.selected_branch)"
        >
          <RefreshCw :size="15" :class="{ spinning: refreshing }" /> Refresh snapshot
        </button>
        <a class="text-link" :href="project.web_url" target="_blank" rel="noreferrer"
          >GitLab <ArrowUpRight :size="14"
        /></a>
      </section>

      <nav class="project-tabs" aria-label="Project areas">
        <button
          v-for="tab in tabs"
          :key="tab"
          type="button"
          :class="{ active: activeTab === tab }"
          @click="activeTab = tab"
        >
          {{ tab }}
        </button>
      </nav>

      <section v-if="activeTab === 'overview'" class="project-overview-grid">
        <article class="project-card hero-card">
          <span class="project-card-icon"><Bot :size="22" /></span>
          <div>
            <p class="eyebrow">Project agent</p>
            <h2>Repository context, automatically selected</h2>
            <p>
              Every turn is bound to {{ project.selected_branch }} at
              {{ shortCommit(project.pinned_commit) }} and current Connector authority.
            </p>
          </div>
          <button class="button primary" type="button" @click="activeTab = 'chat'">
            Open chat
          </button>
        </article>
        <article class="project-card">
          <FileCode2 :size="21" /><strong>Snapshot</strong>
          <p>{{ project.selected_branch }} · {{ shortCommit(project.pinned_commit) }}</p>
        </article>
        <article class="project-card">
          <MessageSquareText :size="21" /><strong>Personal threads</strong>
          <p>{{ threads.length }} durable conversation{{ threads.length === 1 ? "" : "s" }}</p>
        </article>
        <article class="project-card">
          <Boxes :size="21" /><strong>Pre-built workflows</strong>
          <p>{{ workflows.length }} analysis workflows enabled</p>
        </article>
      </section>

      <section v-else-if="activeTab === 'files'" class="project-surface repository-tree-surface">
        <header>
          <div>
            <p class="eyebrow">Governed repository preview</p>
            <h2>Root tree at {{ shortCommit(project.pinned_commit) }}</h2>
          </div>
          <span class="status-pill neutral">Read only</span>
        </header>
        <div class="repository-tree" role="list">
          <div v-for="entry in repositoryTree" :key="entry.object_id" role="listitem">
            <FolderGit2 v-if="entry.kind === 'tree'" :size="18" />
            <FileCode2 v-else :size="18" />
            <strong>{{ entry.name }}</strong>
            <small>{{ entry.kind }} · {{ entry.mode }}</small>
          </div>
          <p v-if="!repositoryTree.length">The exact snapshot contains no root entries.</p>
        </div>
        <p class="surface-note">
          This preview is read through the current Connector grant. A populated Substrate filesystem
          remains a separate materialization step.
        </p>
      </section>

      <section v-else-if="activeTab === 'chat'" class="project-chat">
        <aside class="thread-rail">
          <div>
            <p class="eyebrow">Your threads</p>
            <strong>{{ project.selected_branch }}</strong>
          </div>
          <button
            v-for="thread in threads"
            :key="thread.id"
            type="button"
            :class="{ active: selectedThreadId === thread.id }"
            @click="selectedThreadId = thread.id"
          >
            <MessageSquareText :size="15" /><span>{{ thread.title }}</span
            ><small>{{ shortCommit(thread.pinned_commit) }}</small>
          </button>
          <p v-if="!threads.length">Your first message creates a private thread at this commit.</p>
        </aside>
        <div class="chat-console">
          <div class="message-stream">
            <div v-if="!messages.length" class="chat-empty">
              <Bot :size="30" />
              <h2>Ask about this repository</h2>
              <p>The agent receives only bounded read tools and the exact commit shown above.</p>
            </div>
            <article
              v-for="message in messages"
              :key="message.sequence"
              class="project-message"
              :class="message.role"
            >
              <header>
                <strong>{{
                  message.role === "user"
                    ? "You"
                    : message.role === "assistant"
                      ? "Project agent"
                      : "Snapshot"
                }}</strong
                ><span>{{ shortCommit(message.commit) }}</span>
              </header>
              <p>{{ message.content }}</p>
            </article>
          </div>
          <form class="project-composer" @submit.prevent="sendMessage">
            <textarea
              v-model="draft"
              rows="3"
              placeholder="Ask about architecture, behavior, risks, or a file…"
            ></textarea
            ><button
              class="button primary"
              type="submit"
              :disabled="sending || !draft.trim() || !project.pinned_commit"
            >
              <LoaderCircle v-if="sending" class="spinning" :size="16" /><Bot v-else :size="16" />
              Send to project agent
            </button>
          </form>
        </div>
      </section>

      <section v-else-if="activeTab === 'workflows'" class="workflow-grid">
        <article v-for="definition in workflows" :key="definition.id" class="workflow-card">
          <span class="project-card-icon"
            ><ShieldCheck v-if="definition.id.includes('security')" :size="21" /><Boxes
              v-else
              :size="21"
          /></span>
          <div>
            <span class="status-pill">{{ definition.id }}</span>
            <h2>{{ definition.name }}</h2>
            <p>{{ definition.description }}</p>
          </div>
          <button
            class="button primary"
            type="button"
            :disabled="runningWorkflow === definition.id || !project.pinned_commit"
            @click="startWorkflow(definition)"
          >
            <LoaderCircle
              v-if="runningWorkflow === definition.id"
              class="spinning"
              :size="15"
            /><Play v-else :size="15" /> Run at {{ shortCommit(project.pinned_commit) }}
          </button>
        </article>
        <article v-for="run in runs" :key="run.id" class="workflow-run">
          <span class="status-pill" :class="run.state">{{ run.state }}</span
          ><strong>{{ run.definition_id }}</strong
          ><span>{{ run.branch }} · {{ shortCommit(run.commit) }}</span>
        </article>
      </section>

      <section v-else class="project-surface aep-surface">
        <header class="aep-heading">
          <Boxes :size="30" />
          <div>
            <p class="eyebrow">Central engineering plan</p>
            <h2>AEP drafts and ESS evidence</h2>
            <p>
              Authorized entities are read from the central AEP authority and indexed to this
              canonical project. Lifecycle promotion remains an explicit decision.
            </p>
          </div>
        </header>
        <div v-if="aepError" class="inline-state error-state" role="alert">
          <ShieldCheck :size="19" />
          <div>
            <strong>AEP projection unavailable</strong>
            <p>{{ aepError }}</p>
          </div>
        </div>
        <div v-else-if="artifacts.length" class="aep-artifact-list">
          <article v-for="artifact in artifacts" :key="artifact.id" class="aep-artifact-card">
            <div>
              <span class="status-pill">{{ artifact.entity_type }}</span>
              <h3>{{ artifact.title || artifact.locator }}</h3>
              <p>{{ artifact.locator }}</p>
            </div>
            <dl>
              <div>
                <dt>Revision</dt>
                <dd>{{ artifact.revision }}</dd>
              </div>
              <div v-if="artifact.status">
                <dt>Status</dt>
                <dd>{{ artifact.status }}</dd>
              </div>
              <div v-if="artifact.source_revision">
                <dt>Source</dt>
                <dd>{{ shortCommit(artifact.source_revision) }}</dd>
              </div>
            </dl>
          </article>
          <p v-if="artifactsHaveMore" class="project-note">
            More central entities exist beyond this bounded page.
          </p>
        </div>
        <span v-else class="status-pill neutral">No central artifacts indexed to this project</span>
      </section>
    </template>
  </div>
</template>
