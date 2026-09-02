<script setup lang="ts">
import {
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  BookOpen,
  CircleAlert,
  KeyRound,
  RefreshCw,
  Search,
} from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  api,
  errorMessage,
  type ConnectSession,
  type ConnectorCatalogOperation,
  type ConnectorCatalogPage,
  type ConnectorProviderDescription,
  type ConnectorProviderSummary,
  type ConnectorSetupProfile,
} from "@/api/client";
import ConnectionsView from "@/features/connections/ConnectionsView.vue";
import { useWorkspaceStore } from "@/stores/workspace";

type ConnectorTab = "connections" | "catalog";
type ExposureFilter = "all" | "exposed" | "hidden";

const route = useRoute();
const router = useRouter();
const workspace = useWorkspaceStore();
const search = ref(typeof route.query.q === "string" ? route.query.q : "");
const activeQuery = ref(search.value.trim());
const offset = ref(0);
const catalog = ref<ConnectorCatalogPage>();
const catalogLoading = ref(false);
const catalogError = ref("");
const detail = ref<ConnectorProviderDescription>();
const detailLoading = ref(false);
const detailError = ref("");
const operationSearch = ref("");
const serviceFilter = ref("all");
const riskFilter = ref("all");
const exposureFilter = ref<ExposureFilter>("all");
const startingProfile = ref("");
const setupSessions = ref<Record<string, ConnectSession>>({});
const setupError = ref("");

const providerRef = computed(() =>
  typeof route.params.providerRef === "string" ? route.params.providerRef : undefined,
);
const activeTab = computed<ConnectorTab>(() => {
  if (providerRef.value || route.query.tab === "catalog" || typeof route.query.q === "string") {
    return "catalog";
  }
  return "connections";
});
const services = computed(() => [
  ...new Set(detail.value?.operations.map((operation) => operation.service) ?? []),
]);
const risks = computed(() => [
  ...new Set(detail.value?.operations.map((operation) => operation.risk) ?? []),
]);
const filteredOperations = computed(() => {
  const query = operationSearch.value.trim().toLowerCase();
  return (detail.value?.operations ?? []).filter((operation) => {
    const matchesQuery =
      !query ||
      operation.operation_ref.toLowerCase().includes(query) ||
      operation.description.toLowerCase().includes(query);
    const matchesService =
      serviceFilter.value === "all" || operation.service === serviceFilter.value;
    const matchesRisk = riskFilter.value === "all" || operation.risk === riskFilter.value;
    const matchesExposure =
      exposureFilter.value === "all" ||
      (exposureFilter.value === "exposed" ? operation.exposed : !operation.exposed);
    return matchesQuery && matchesService && matchesRisk && matchesExposure;
  });
});

onMounted(() => void loadCurrentSurface());
watch(providerRef, () => void loadCurrentSurface());
watch(activeTab, () => void loadCurrentSurface());

async function loadCurrentSurface() {
  if (activeTab.value !== "catalog") return;
  if (providerRef.value) await loadProvider(providerRef.value);
  else await loadCatalog();
}

async function loadCatalog() {
  catalogLoading.value = true;
  catalogError.value = "";
  try {
    catalog.value = await api.connectorCatalog(activeQuery.value, offset.value);
  } catch (cause) {
    catalogError.value = errorMessage(cause);
  } finally {
    catalogLoading.value = false;
  }
}

async function loadProvider(reference: string) {
  detailLoading.value = true;
  detailError.value = "";
  detail.value = undefined;
  resetOperationFilters();
  try {
    detail.value = await api.connectorCatalogProvider(reference);
  } catch (cause) {
    detailError.value = errorMessage(cause);
  } finally {
    detailLoading.value = false;
  }
}

async function submitSearch() {
  activeQuery.value = search.value.trim();
  offset.value = 0;
  await router.replace({
    path: "/connectors",
    query: activeQuery.value ? { tab: "catalog", q: activeQuery.value } : { tab: "catalog" },
  });
  await loadCatalog();
}

async function changePage(nextOffset: number) {
  offset.value = Math.max(0, nextOffset);
  await loadCatalog();
  window.scrollTo({ top: 0, behavior: "smooth" });
}

async function selectTab(tab: ConnectorTab) {
  await router.push({
    path: "/connectors",
    query: tab === "catalog" ? { tab: "catalog" } : {},
  });
}

async function returnToCatalog() {
  await router.push({
    path: "/connectors",
    query: activeQuery.value ? { tab: "catalog", q: activeQuery.value } : { tab: "catalog" },
  });
}

function resetOperationFilters() {
  operationSearch.value = "";
  serviceFilter.value = "all";
  riskFilter.value = "all";
  exposureFilter.value = "all";
}

function setupKey(provider: ConnectorProviderSummary, profile: ConnectorSetupProfile): string {
  return `${provider.provider_ref}:${profile.auth_profile}`;
}

function setupLabel(profile: ConnectorSetupProfile): string {
  const label = profile.auth_profile.split(".").at(-1)?.replaceAll("_", " ") ?? "connection";
  return label.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function setupState(provider: ConnectorProviderSummary, profile: ConnectorSetupProfile) {
  return setupSessions.value[setupKey(provider, profile)];
}

async function startSetup(provider: ConnectorProviderSummary, profile: ConnectorSetupProfile) {
  const key = setupKey(provider, profile);
  startingProfile.value = key;
  setupError.value = "";
  try {
    const session = await api.startConnection(
      provider.provider_ref,
      `My ${provider.vendor}`,
      profile.auth_profile,
    );
    setupSessions.value = { ...setupSessions.value, [key]: session };
    if (session.browser_completion_url) {
      window.open(session.browser_completion_url, "_blank", "noopener,noreferrer");
    }
    if (session.state === "pending") void pollSetup(key, session.connect_session_ref, 0);
  } catch (cause) {
    setupError.value = errorMessage(cause);
  } finally {
    startingProfile.value = "";
  }
}

async function pollSetup(key: string, sessionRef: string, attempt: number) {
  if (attempt >= 60) return;
  await new Promise((resolve) => window.setTimeout(resolve, 2_000));
  try {
    const session = await api.connectionSession(sessionRef);
    setupSessions.value = { ...setupSessions.value, [key]: session };
    if (session.state === "pending") void pollSetup(key, sessionRef, attempt + 1);
  } catch (cause) {
    setupError.value = errorMessage(cause);
  }
}

function exposureLabel(operation: ConnectorCatalogOperation): string {
  return operation.exposed ? "Model exposed" : "Catalog only";
}
</script>

<template>
  <div class="view connectors-view">
    <header class="view-header connector-heading">
      <div>
        <p class="eyebrow">Connector workspace</p>
        <h1>Connectors</h1>
        <p>
          Manage your connections first, then explore the deployment catalog and its operation
          contracts when you need something new.
        </p>
      </div>
      <a
        v-if="workspace.session?.connectors_docs_available"
        class="button quiet"
        href="/api/connectors/v1/docs"
        target="_blank"
        rel="noopener noreferrer"
      >
        <BookOpen :size="17" /> Connector API <ArrowUpRight :size="15" />
      </a>
    </header>

    <nav class="connector-tabs" aria-label="Connector workspace">
      <button
        type="button"
        :class="{ active: activeTab === 'connections' }"
        :aria-current="activeTab === 'connections' ? 'page' : undefined"
        @click="selectTab('connections')"
      >
        My connectors
      </button>
      <button
        type="button"
        :class="{ active: activeTab === 'catalog' }"
        :aria-current="activeTab === 'catalog' ? 'page' : undefined"
        @click="selectTab('catalog')"
      >
        Catalog
      </button>
    </nav>

    <ConnectionsView v-if="activeTab === 'connections'" embedded />

    <main v-else-if="providerRef" class="connector-detail">
      <button class="connector-back" type="button" @click="returnToCatalog">
        <ArrowLeft :size="16" /> Back to catalog
      </button>
      <div v-if="detailLoading" class="catalog-loading" aria-live="polite">
        <RefreshCw class="spinning" :size="22" /> Loading provider contract…
      </div>
      <div v-else-if="detailError" class="catalog-error" role="alert">
        <CircleAlert :size="21" />
        <div>
          <strong>Provider unavailable</strong>
          <p>{{ detailError }}</p>
        </div>
        <button class="button small" type="button" @click="loadProvider(providerRef)">Retry</button>
      </div>
      <template v-else-if="detail">
        <section class="provider-detail-hero">
          <span class="catalog-provider-mark">{{ detail.provider.vendor[0]?.toUpperCase() }}</span>
          <div>
            <p class="eyebrow">{{ detail.provider.authority ?? detail.provider.provider_ref }}</p>
            <h2>{{ detail.provider.vendor }}</h2>
            <p>{{ detail.provider.description }}</p>
            <div class="catalog-chips">
              <span v-for="service in detail.provider.services" :key="service">{{ service }}</span>
              <span>{{ detail.provider.operation_count }} operations</span>
            </div>
          </div>
          <div v-if="detail.provider.setup_profiles.length" class="provider-setup-panel">
            <strong>Connect {{ detail.provider.vendor }}</strong>
            <p>Credentials stay inside Connector custody.</p>
            <button
              v-for="profile in detail.provider.setup_profiles"
              :key="profile.auth_profile"
              class="button small"
              :class="{ primary: profile.actor === 'person' }"
              type="button"
              :disabled="
                startingProfile === setupKey(detail.provider, profile) ||
                setupState(detail.provider, profile)?.state === 'pending'
              "
              @click="startSetup(detail.provider, profile)"
            >
              <KeyRound :size="15" />
              {{
                setupState(detail.provider, profile)?.state === "pending"
                  ? "Waiting for provider…"
                  : setupState(detail.provider, profile)?.state === "completed"
                    ? "Connected"
                    : setupLabel(profile)
              }}
            </button>
          </div>
        </section>
        <p v-if="setupError" class="form-error catalog-setup-error" role="alert">
          <CircleAlert :size="16" /> {{ setupError }}
        </p>

        <section class="operation-index">
          <header>
            <div>
              <p class="eyebrow">Operation index</p>
              <h2>Declared capabilities</h2>
            </div>
            <span>{{ filteredOperations.length }} of {{ detail.operations.length }}</span>
          </header>
          <div class="operation-filters">
            <label class="catalog-search compact">
              <Search :size="16" />
              <span class="sr-only">Filter operations</span>
              <input v-model="operationSearch" type="search" placeholder="Filter operations" />
            </label>
            <select v-model="serviceFilter" aria-label="Filter by service">
              <option value="all">All services</option>
              <option v-for="service in services" :key="service" :value="service">
                {{ service }}
              </option>
            </select>
            <select v-model="riskFilter" aria-label="Filter by risk">
              <option value="all">All risks</option>
              <option v-for="risk in risks" :key="risk" :value="risk">{{ risk }}</option>
            </select>
            <select v-model="exposureFilter" aria-label="Filter by exposure">
              <option value="all">All exposure</option>
              <option value="exposed">Model exposed</option>
              <option value="hidden">Catalog only</option>
            </select>
          </div>
          <div class="operation-list">
            <article v-for="operation in filteredOperations" :key="operation.operation_ref">
              <div>
                <code>{{ operation.operation_ref }}</code>
                <p>{{ operation.description }}</p>
              </div>
              <div class="operation-meta">
                <span>{{ operation.service }}</span>
                <span>{{ operation.risk }}</span>
                <span :class="{ exposed: operation.exposed }">{{ exposureLabel(operation) }}</span>
              </div>
            </article>
            <p v-if="filteredOperations.length === 0" class="provider-empty">
              No operations match these filters.
            </p>
          </div>
        </section>
      </template>
    </main>

    <main v-else class="catalog-surface">
      <form class="catalog-toolbar" role="search" @submit.prevent="submitSearch">
        <label class="catalog-search">
          <Search :size="18" />
          <span class="sr-only">Search Connector catalog</span>
          <input v-model="search" type="search" placeholder="Search providers and capabilities" />
        </label>
        <button class="button primary" type="submit" :disabled="catalogLoading">
          Search catalog
        </button>
      </form>

      <div v-if="catalogLoading" class="catalog-loading" aria-live="polite">
        <RefreshCw class="spinning" :size="22" /> Loading Connector catalog…
      </div>
      <div v-else-if="catalogError" class="catalog-error" role="alert">
        <CircleAlert :size="21" />
        <div>
          <strong>Catalog unavailable</strong>
          <p>{{ catalogError }}</p>
        </div>
        <button class="button small" type="button" @click="loadCatalog">Retry</button>
      </div>
      <template v-else>
        <div class="catalog-grid">
          <article
            v-for="provider in catalog?.providers ?? []"
            :key="provider.provider_ref"
            class="catalog-provider-card"
          >
            <RouterLink :to="`/connectors/${encodeURIComponent(provider.provider_ref)}`">
              <header>
                <span class="catalog-provider-mark">{{ provider.vendor[0]?.toUpperCase() }}</span>
                <div>
                  <h2>{{ provider.vendor }}</h2>
                  <code>{{ provider.provider_ref }}</code>
                </div>
                <ArrowRight :size="17" />
              </header>
              <p>{{ provider.description }}</p>
              <div class="catalog-chips">
                <span v-for="service in provider.services.slice(0, 3)" :key="service">{{
                  service
                }}</span>
                <span>{{ provider.operation_count }} operations</span>
              </div>
            </RouterLink>
            <footer v-if="provider.setup_profiles.length">
              <span>Setup available</span>
              <button
                v-for="profile in provider.setup_profiles"
                :key="profile.auth_profile"
                class="button quiet small"
                type="button"
                :disabled="
                  startingProfile === setupKey(provider, profile) ||
                  setupState(provider, profile)?.state === 'pending'
                "
                @click="startSetup(provider, profile)"
              >
                <KeyRound :size="14" />
                {{
                  setupState(provider, profile)?.state === "pending"
                    ? "Waiting…"
                    : setupState(provider, profile)?.state === "completed"
                      ? "Connected"
                      : setupLabel(profile)
                }}
              </button>
            </footer>
          </article>
        </div>
        <p v-if="(catalog?.providers.length ?? 0) === 0" class="catalog-empty">
          No providers match <strong>{{ activeQuery || "this search" }}</strong
          >.
        </p>
        <p v-if="setupError" class="form-error catalog-setup-error" role="alert">
          <CircleAlert :size="16" /> {{ setupError }}
        </p>
        <nav class="catalog-pagination" aria-label="Catalog pages">
          <button
            class="button quiet small"
            type="button"
            :disabled="offset === 0 || catalogLoading"
            @click="changePage(Math.max(0, offset - 24))"
          >
            <ArrowLeft :size="15" /> Previous
          </button>
          <span>Showing from {{ offset + 1 }}</span>
          <button
            class="button quiet small"
            type="button"
            :disabled="catalog?.next_offset == null || catalogLoading"
            @click="changePage(catalog?.next_offset ?? offset)"
          >
            Next <ArrowRight :size="15" />
          </button>
        </nav>
      </template>
    </main>
  </div>
</template>
