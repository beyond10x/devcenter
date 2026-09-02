<script setup lang="ts">
import {
  Bot,
  BookOpen,
  Cable,
  FolderGit2,
  LogOut,
  PanelLeftClose,
  PanelLeftOpen,
  RadioTower,
  ShieldCheck,
  X,
} from "@lucide/vue";
import { ref } from "vue";
import { useWorkspaceStore } from "@/stores/workspace";

const workspace = useWorkspaceStore();
const navigationOpen = ref(false);
const reviewMode = import.meta.env.MODE === "review";

const navigation = [
  { to: "/projects", label: "Projects", icon: FolderGit2 },
  { to: "/agents", label: "Agents", icon: Bot },
  { to: "/connections", label: "Connections", icon: Cable },
  { to: "/profiles", label: "Capability profiles", icon: ShieldCheck },
  { to: "/publications", label: "MCP publications", icon: RadioTower },
  { to: "/docs", label: "Documentation", icon: BookOpen },
];

async function logout() {
  await workspace.logout();
}
</script>

<template>
  <div class="app-frame">
    <button
      class="mobile-nav-trigger icon-button"
      type="button"
      :aria-expanded="navigationOpen"
      aria-label="Open navigation"
      @click="navigationOpen = true"
    >
      <PanelLeftOpen :size="20" />
    </button>
    <div v-if="navigationOpen" class="nav-scrim" @click="navigationOpen = false"></div>
    <aside class="app-sidebar" :class="{ open: navigationOpen }">
      <div class="sidebar-top">
        <RouterLink to="/agents" class="brand-lockup" @click="navigationOpen = false">
          <span class="brand-glyph" aria-hidden="true">D</span><span>Devcenter</span>
        </RouterLink>
        <button
          class="icon-button mobile-only"
          type="button"
          aria-label="Close navigation"
          @click="navigationOpen = false"
        >
          <PanelLeftClose :size="19" />
        </button>
      </div>
      <nav aria-label="Primary navigation">
        <p class="nav-label">Workspace</p>
        <RouterLink
          v-for="item in navigation"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          @click="navigationOpen = false"
        >
          <component :is="item.icon" :size="18" />
          <span>{{ item.label }}</span>
          <span
            v-if="item.to === '/connections'"
            class="nav-status"
            :class="{ ok: workspace.connected }"
          ></span>
        </RouterLink>
      </nav>
      <div class="sidebar-context">
        <p class="nav-label">Authority</p>
        <div class="authority-summary">
          <span class="avatar">{{
            (workspace.session?.email || workspace.session?.subject || "?")[0]?.toUpperCase()
          }}</span>
          <div>
            <strong>{{ workspace.session?.email || workspace.session?.subject }}</strong>
            <span>Verified session</span>
          </div>
        </div>
        <button class="sidebar-logout" type="button" @click="logout">
          <LogOut :size="15" /> Log out browser session
        </button>
      </div>
      <a class="sidebar-api-link" href="/openapi.json">OpenAPI 3.1 <span>↗</span></a>
    </aside>

    <section class="app-main">
      <div v-if="reviewMode" class="review-banner" role="status">
        Local review mode · sample data only · nothing is persisted
      </div>
      <div v-if="workspace.notice" class="toast" role="status">
        <span>{{ workspace.notice }}</span>
        <button type="button" aria-label="Dismiss notification" @click="workspace.clearNotice">
          <X :size="16" />
        </button>
      </div>
      <RouterView />
    </section>
  </div>
</template>
