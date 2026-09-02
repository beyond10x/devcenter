<script setup lang="ts">
import {
  ServiceConsole,
  createHttpServiceBinding,
  type ServiceCatalog as GeneratedServiceCatalog,
} from "@b10x/service-console-vue";
import { Boxes, RefreshCw } from "@lucide/vue";
import { onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { api, errorMessage, type GeneratedServiceSummary } from "@/api/client";

const route = useRoute();
const router = useRouter();
const services = ref<GeneratedServiceSummary[]>([]);
const selected = ref<string | undefined>(
  typeof route.query.service === "string" ? route.query.service : undefined,
);
const catalog = ref<GeneratedServiceCatalog>();
const loading = ref(false);
const failure = ref<string>();
const binding = createHttpServiceBinding({ endpoint: "/api/services/invoke" });

async function loadServices() {
  loading.value = true;
  failure.value = undefined;
  try {
    const page = await api.generatedServices();
    services.value = page.services;
    const requested = typeof route.query.service === "string" ? route.query.service : undefined;
    const initial =
      services.value.find((service) => service.service_ref === requested) ?? services.value[0];
    if (initial) await selectService(initial.service_ref, false);
  } catch (error) {
    failure.value = errorMessage(error);
  } finally {
    loading.value = false;
  }
}

async function selectService(serviceRef: string, updateRoute = true) {
  selected.value = serviceRef;
  if (updateRoute) await router.replace({ path: "/services", query: { service: serviceRef } });
  catalog.value = undefined;
  failure.value = undefined;
  try {
    catalog.value = await api.generatedServiceCatalog(serviceRef);
  } catch (error) {
    failure.value = errorMessage(error);
  }
}

onMounted(loadServices);
watch(
  () => route.query.service,
  (serviceRef) => {
    if (
      typeof serviceRef === "string" &&
      serviceRef !== selected.value &&
      services.value.some((service) => service.service_ref === serviceRef)
    ) {
      void selectService(serviceRef, false);
    }
  },
);
</script>

<template>
  <main class="services-view">
    <header class="page-header">
      <div>
        <p class="eyebrow">Synthesized applications</p>
        <h1>Services</h1>
        <p>
          Exact generated interaction surfaces, bound to your verified Devcenter session and the
          composed service runtime.
        </p>
      </div>
      <button class="button quiet" type="button" :disabled="loading" @click="loadServices">
        <RefreshCw :size="16" /> Refresh
      </button>
    </header>

    <div v-if="failure" class="empty-state" role="alert">
      <strong>Services are unavailable</strong>
      <span>{{ failure }}</span>
    </div>

    <div v-else-if="loading && services.length === 0" class="empty-state" role="status">
      <span class="spinner"></span>
      <span>Loading generated service catalogs…</span>
    </div>

    <div v-else-if="services.length === 0" class="empty-state">
      <Boxes :size="30" />
      <strong>No generated services are deployed</strong>
      <span
        >A composition must register a catalog factory and activate its deployment overlay.</span
      >
    </div>

    <template v-else>
      <nav class="service-picker" aria-label="Generated services">
        <button
          v-for="service in services"
          :key="service.service_ref"
          type="button"
          :class="{ selected: selected === service.service_ref }"
          @click="selectService(service.service_ref)"
        >
          <span>
            <strong>{{ service.display_name }}</strong>
            <small>{{ service.description }}</small>
          </span>
          <code>{{ service.service_ref }}</code>
        </button>
      </nav>

      <ServiceConsole v-if="catalog" :catalog="catalog" :binding="binding" mode="live" />
      <div v-else class="empty-state" role="status">
        <span class="spinner"></span>
        <span>Loading the exact generated console…</span>
      </div>
    </template>
  </main>
</template>

<style scoped>
.services-view {
  display: grid;
  gap: 1.25rem;
  padding: clamp(1rem, 3vw, 2.25rem);
}

.page-header {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 1.5rem;
}

.page-header h1 {
  margin: 0.25rem 0;
  font-size: clamp(2rem, 4vw, 3.25rem);
}

.page-header p:last-child {
  max-width: 66ch;
  margin-bottom: 0;
  color: var(--muted);
}

.service-picker {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
  gap: 0.75rem;
}

.service-picker button {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem;
  text-align: left;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 12px;
  cursor: pointer;
}

.service-picker button.selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 18%, transparent);
}

.service-picker span {
  display: grid;
  gap: 0.3rem;
}

.service-picker small,
.service-picker code {
  color: var(--muted);
}

.service-picker code {
  white-space: nowrap;
  font-size: 0.72rem;
}

@media (max-width: 720px) {
  .page-header {
    align-items: start;
    flex-direction: column;
  }

  .service-picker button {
    flex-direction: column;
  }
}
</style>
