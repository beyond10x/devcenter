<script setup lang="ts">
import {
  Bot,
  BookOpen,
  Cable,
  FolderGit2,
  RadioTower,
  Search,
  ShieldCheck,
  Shapes,
  X,
} from "@lucide/vue";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { api, errorMessage } from "@/api/client";
import { navigationItems } from "@/app/navigation";
import { rankSearchEntries, type SearchEntry, type SearchGroup } from "@/app/search";

const emit = defineEmits<{ close: [] }>();
const router = useRouter();
const input = ref<HTMLInputElement>();
const dialog = ref<HTMLElement>();
const query = ref("");
const entries = ref<SearchEntry[]>(staticEntries());
const selectedIndex = ref(0);
const loadingSources = ref(0);
const failedSources = ref<string[]>([]);
let remoteTimer: number | undefined;
let remoteRequest = 0;

const groupIcons: Record<SearchGroup, typeof Search> = {
  Sections: Search,
  "Projects and repositories": FolderGit2,
  Agents: Bot,
  "Connections and connectors": Cable,
  "Capability profiles": ShieldCheck,
  Services: Shapes,
  "MCP publications": RadioTower,
  Documentation: BookOpen,
};

const results = computed(() => rankSearchEntries(entries.value, query.value));
const groupedResults = computed(() => {
  const groups = new Map<SearchGroup, ReturnType<typeof rankSearchEntries>>();
  for (const result of results.value) {
    const group = groups.get(result.group) ?? [];
    group.push(result);
    groups.set(result.group, group);
  }
  return [...groups.entries()];
});
const activeResult = computed(() => results.value[selectedIndex.value]);
const failedSourceNames = computed(() =>
  failedSources.value.map((failure) => failure.split(":", 1)[0]).join(", "),
);

watch(query, () => {
  selectedIndex.value = 0;
  remoteRequest += 1;
  if (remoteTimer !== undefined) window.clearTimeout(remoteTimer);
  if (query.value.trim().length < 2) {
    entries.value = entries.value.filter(
      (entry) => !entry.key.startsWith("repository:") && !entry.key.startsWith("provider:"),
    );
    return;
  }
  remoteTimer = window.setTimeout(() => void loadRemote(query.value.trim()), 180);
});

onMounted(() => {
  void nextTick(() => input.value?.focus());
  void loadLocalSources();
});
onBeforeUnmount(() => {
  if (remoteTimer !== undefined) window.clearTimeout(remoteTimer);
  remoteRequest += 1;
});

function staticEntries(): SearchEntry[] {
  const sections = navigationItems.map((item) => ({
    key: `section:${item.id}`,
    group: "Sections" as const,
    label: item.label,
    detail: `Go to ${item.label} · G ${item.chord.toUpperCase()}`,
    to: item.to,
    terms: item.searchTerms,
  }));
  return [
    ...sections,
    {
      key: "docs:authority",
      group: "Documentation",
      label: "Authority model",
      detail: "Documentation",
      to: { path: "/docs", hash: "#sign-in" },
      terms: "tenant user realm session",
    },
    {
      key: "docs:connectors",
      group: "Documentation",
      label: "Connector custody",
      detail: "Documentation",
      to: { path: "/docs", hash: "#connect-claude-code" },
      terms: "connections credentials grants",
    },
    {
      key: "docs:services",
      group: "Documentation",
      label: "Generated services",
      detail: "Documentation",
      to: { path: "/docs", hash: "#services" },
      terms: "sdk ess console",
    },
  ];
}

async function loadLocalSources() {
  const sources = [
    {
      name: "agents",
      load: async () =>
        (await api.agents()).map<SearchEntry>((agent) => ({
          key: `agent:${agent.id}`,
          group: "Agents",
          label: agent.name,
          detail: `Agent · revision ${String(agent.active_revision ?? agent.latest_revision)}`,
          to: { name: "agent", params: { agentId: agent.id } },
          terms: `${agent.id} ${agent.created_by}`,
        })),
    },
    {
      name: "connections",
      load: async () =>
        (await api.connections()).map<SearchEntry>((connection) => ({
          key: `connection:${connection.connection_ref}`,
          group: "Connections and connectors",
          label: connection.label,
          detail: `${connection.integration_ref} · ${connection.state}`,
          to: {
            path: "/connectors",
            query: { tab: "connections", connection: connection.connection_ref },
          },
          terms: `${connection.connection_ref} ${connection.integration_ref} ${connection.auth_profile ?? ""}`,
        })),
    },
    {
      name: "capability profiles",
      load: async () =>
        (await api.capabilityProfiles()).map<SearchEntry>((profile) => ({
          key: `profile:${profile.id}`,
          group: "Capability profiles",
          label: profile.name,
          detail: `Capability profile · revision ${String(profile.revision)}`,
          to: { path: "/profiles", query: { profile: profile.id } },
          terms: profile.id,
        })),
    },
    {
      name: "services",
      load: async () =>
        (await api.generatedServices()).services.map<SearchEntry>((service) => ({
          key: `service:${service.service_ref}`,
          group: "Services",
          label: service.display_name,
          detail: service.description,
          to: { path: "/services", query: { service: service.service_ref } },
          terms: `${service.service_ref} ${service.digest}`,
        })),
    },
    {
      name: "MCP publications",
      load: async () =>
        (await api.publications()).map<SearchEntry>((publication) => ({
          key: `publication:${publication.publication_id}`,
          group: "MCP publications",
          label: publication.profile_id,
          detail: `MCP publication · ${publication.state}`,
          to: { path: "/publications", query: { publication: publication.publication_id } },
          terms: `${publication.publication_id} ${publication.toolset_digest}`,
        })),
    },
  ];
  loadingSources.value += sources.length;
  await Promise.all(
    sources.map(async (source) => {
      try {
        const loaded = await source.load();
        entries.value = [
          ...entries.value.filter((entry) => !loaded.some((item) => item.key === entry.key)),
          ...loaded,
        ];
      } catch (caught) {
        recordSourceFailure(source.name, caught);
      } finally {
        loadingSources.value -= 1;
      }
    }),
  );
}

async function loadRemote(needle: string) {
  const request = ++remoteRequest;
  loadingSources.value += 2;
  const [repositories, providers] = await Promise.allSettled([
    api.repositories(needle),
    api.connectorCatalog(needle, 0, 8),
  ]);
  loadingSources.value = Math.max(0, loadingSources.value - 2);
  if (request !== remoteRequest) return;
  entries.value = entries.value.filter(
    (entry) => !entry.key.startsWith("repository:") && !entry.key.startsWith("provider:"),
  );
  if (repositories.status === "fulfilled") {
    clearSourceFailure("repositories");
    entries.value.push(
      ...repositories.value.map<SearchEntry>((repository) => ({
        key: `repository:${repository.forge_instance_ref}:${repository.project_ref}`,
        group: "Projects and repositories",
        label: repository.path_with_namespace,
        detail: repository.opened_project_id
          ? "Opened project"
          : `Repository · ${repository.visibility}`,
        to: repository.opened_project_id
          ? { name: "project", params: { projectId: repository.opened_project_id } }
          : { path: "/projects", query: { q: repository.path_with_namespace } },
        terms: `${repository.name} ${repository.default_branch ?? ""}`,
      })),
    );
  } else {
    recordSourceFailure("repositories", repositories.reason);
  }
  if (providers.status === "fulfilled") {
    clearSourceFailure("connector catalog");
    entries.value.push(
      ...providers.value.providers.map<SearchEntry>((provider) => ({
        key: `provider:${provider.provider_ref}`,
        group: "Connections and connectors",
        label: provider.vendor,
        detail: provider.description,
        to: { name: "connector", params: { providerRef: provider.provider_ref } },
        terms: `${provider.provider_ref} ${provider.services.join(" ")}`,
      })),
    );
  } else {
    recordSourceFailure("connector catalog", providers.reason);
  }
}

function clearSourceFailure(source: string) {
  failedSources.value = failedSources.value.filter((failure) => !failure.startsWith(`${source}:`));
}

function recordSourceFailure(source: string, caught: unknown) {
  clearSourceFailure(source);
  failedSources.value.push(`${source}: ${errorMessage(caught)}`);
}

function moveSelection(delta: number) {
  if (!results.value.length) return;
  selectedIndex.value = (selectedIndex.value + delta + results.value.length) % results.value.length;
  document
    .getElementById(`search-result-${String(selectedIndex.value)}`)
    ?.scrollIntoView({ block: "nearest" });
}

async function activate(entry = activeResult.value) {
  if (!entry) return;
  emit("close");
  await router.push(entry.to);
}

function trapFocus(event: KeyboardEvent) {
  if (event.key !== "Tab" || !dialog.value) return;
  const focusable = [...dialog.value.querySelectorAll<HTMLElement>("button, input")];
  const first = focusable[0];
  const last = focusable.at(-1);
  if (!first || !last) return;
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="overlay-layer search-overlay" role="presentation" @click.self="emit('close')">
      <section
        ref="dialog"
        class="global-search-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="search-title"
        @keydown="trapFocus"
        @keydown.esc.stop="emit('close')"
      >
        <header class="search-dialog-header">
          <Search :size="21" aria-hidden="true" />
          <label id="search-title" class="sr-only" for="global-search-input"
            >Search all Devcenter resources</label
          >
          <input
            id="global-search-input"
            ref="input"
            v-model="query"
            type="search"
            role="combobox"
            aria-autocomplete="list"
            aria-controls="global-search-results"
            :aria-expanded="true"
            :aria-activedescendant="
              activeResult ? `search-result-${String(selectedIndex)}` : undefined
            "
            placeholder="Search projects, agents, connectors, services…"
            autocomplete="off"
            @keydown.down.prevent="moveSelection(1)"
            @keydown.up.prevent="moveSelection(-1)"
            @keydown.enter.prevent="activate()"
          />
          <kbd>Esc</kbd>
          <button
            class="icon-button"
            type="button"
            aria-label="Close search"
            @click="emit('close')"
          >
            <X :size="18" />
          </button>
        </header>
        <p v-if="failedSources.length" class="search-partial" role="status">
          Some sources are unavailable: {{ failedSourceNames }}. Results from other sources remain
          current.
          <span :title="failedSources.join('\n')"
            >{{ failedSources.length }} source{{ failedSources.length === 1 ? "" : "s" }}</span
          >
        </p>
        <div
          id="global-search-results"
          class="global-search-results"
          role="listbox"
          aria-label="Search results"
        >
          <section
            v-for="[group, groupResults] in groupedResults"
            :key="group"
            class="search-result-group"
          >
            <h3>{{ group }}</h3>
            <button
              v-for="entry in groupResults"
              :id="`search-result-${String(results.indexOf(entry))}`"
              :key="entry.key"
              type="button"
              role="option"
              class="search-result"
              :class="{ selected: results.indexOf(entry) === selectedIndex }"
              :aria-selected="results.indexOf(entry) === selectedIndex"
              @mouseenter="selectedIndex = results.indexOf(entry)"
              @click="activate(entry)"
            >
              <span class="search-result-icon"
                ><component :is="groupIcons[group]" :size="17"
              /></span>
              <span
                ><strong>{{ entry.label }}</strong
                ><small>{{ entry.detail }}</small></span
              >
              <kbd>↵</kbd>
            </button>
          </section>
          <div v-if="!results.length && loadingSources" class="search-empty" role="status">
            <span class="spinner"></span><strong>Searching visible resources…</strong>
          </div>
          <div v-else-if="!results.length" class="search-empty">
            <Search :size="26" /><strong>No destination found</strong
            ><span>Try a name, identifier, provider, or section.</span>
          </div>
        </div>
        <footer class="search-hint">
          <span><kbd>↑</kbd><kbd>↓</kbd> navigate</span><span><kbd>Enter</kbd> open</span
          ><span><kbd>Esc</kbd> close</span>
        </footer>
      </section>
    </div>
  </Teleport>
</template>
