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
  Shapes,
  Keyboard,
  Search,
  X,
} from "@lucide/vue";
import type { Component } from "vue";
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import GlobalSearch from "@/app/GlobalSearch.vue";
import KeyboardHelp from "@/app/KeyboardHelp.vue";
import { navigationItems } from "@/app/navigation";
import ThemePicker from "@/app/ThemePicker.vue";
import { useWorkspaceStore } from "@/stores/workspace";

const workspace = useWorkspaceStore();
const router = useRouter();
const navigationOpen = ref(false);
const searchOpen = ref(false);
const helpOpen = ref(false);
const chordOpen = ref(false);
const searchTrigger = ref<HTMLButtonElement>();
const helpTrigger = ref<HTMLButtonElement>();
const reviewMode = import.meta.env.MODE === "review";
const commandKey = /Mac|iPhone|iPad/.test(globalThis.navigator.userAgent) ? "⌘ K" : "Ctrl K";
let chordTimer: number | undefined;

const navigationIcons: Record<string, Component> = {
  projects: FolderGit2,
  agents: Bot,
  connectors: Cable,
  services: Shapes,
  profiles: ShieldCheck,
  publications: RadioTower,
  docs: BookOpen,
};

onMounted(() => window.addEventListener("keydown", onGlobalKeydown));
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  if (chordTimer !== undefined) window.clearTimeout(chordTimer);
});

function isInteractive(target: EventTarget | null): boolean {
  return Boolean(
    target instanceof Element &&
    target.closest("input, textarea, select, [contenteditable='true']"),
  );
}

function beginChord() {
  chordOpen.value = true;
  if (chordTimer !== undefined) window.clearTimeout(chordTimer);
  chordTimer = window.setTimeout(() => {
    chordOpen.value = false;
  }, 1_000);
}

function clearChord() {
  chordOpen.value = false;
  if (chordTimer !== undefined) window.clearTimeout(chordTimer);
  chordTimer = undefined;
}

function onGlobalKeydown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.repeat) return;
  const key = event.key.toLocaleLowerCase();
  if ((event.metaKey || event.ctrlKey) && key === "k") {
    event.preventDefault();
    helpOpen.value = false;
    searchOpen.value = !searchOpen.value;
    clearChord();
    return;
  }
  if (searchOpen.value || helpOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (searchOpen.value) closeSearch();
      else closeHelp();
    }
    return;
  }
  if (isInteractive(event.target) || event.metaKey || event.ctrlKey || event.altKey) return;
  if (event.key === "?") {
    event.preventDefault();
    helpOpen.value = true;
    clearChord();
    return;
  }
  if (chordOpen.value) {
    const destination = navigationItems.find((item) => item.chord === key);
    clearChord();
    if (destination) {
      event.preventDefault();
      navigationOpen.value = false;
      void router.push(destination.to);
    }
    return;
  }
  if (key === "g") {
    event.preventDefault();
    beginChord();
  }
}

function openSearch() {
  helpOpen.value = false;
  searchOpen.value = true;
}

function closeSearch() {
  searchOpen.value = false;
  void nextTick(() => searchTrigger.value?.focus());
}

function openHelp() {
  searchOpen.value = false;
  helpOpen.value = true;
}

function closeHelp() {
  helpOpen.value = false;
  void nextTick(() => helpTrigger.value?.focus());
}

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
          v-for="item in navigationItems"
          :key="item.id"
          :to="item.to"
          class="nav-item"
          :title="`${item.label} (G ${item.chord.toUpperCase()})`"
          @click="navigationOpen = false"
        >
          <span class="nav-icon"><component :is="navigationIcons[item.id]" :size="18" /></span>
          <span>{{ item.label }}</span>
          <span
            v-if="item.to === '/connectors'"
            class="nav-status"
            :class="{ ok: workspace.connected }"
          ></span>
          <span v-if="item.id === 'agents' && workspace.agents.length" class="nav-badge">{{
            workspace.agents.length
          }}</span>
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
        <ThemePicker />
        <button class="sidebar-logout" type="button" @click="logout">
          <LogOut :size="15" /> Log out browser session
        </button>
      </div>
      <a class="sidebar-api-link" href="/openapi.json">OpenAPI 3.1 <span>↗</span></a>
    </aside>

    <section class="app-main">
      <header class="shell-toolbar">
        <button
          ref="searchTrigger"
          class="search-all-trigger"
          type="button"
          aria-haspopup="dialog"
          aria-keyshortcuts="Control+K Meta+K"
          @click="openSearch"
        >
          <Search :size="17" />
          <span>Search all</span>
          <small>Projects, agents, connectors, services…</small>
          <kbd>{{ commandKey }}</kbd>
        </button>
        <button
          ref="helpTrigger"
          class="keyboard-help-trigger icon-button"
          type="button"
          aria-label="Show keyboard shortcuts"
          aria-haspopup="dialog"
          aria-keyshortcuts="Shift+/"
          @click="openHelp"
        >
          <Keyboard :size="18" /><kbd>?</kbd>
        </button>
      </header>
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
    <div v-if="chordOpen" class="chord-hint" role="status">
      <kbd>G</kbd><span>Choose P, A, C, S, F, M, or D</span>
    </div>
    <GlobalSearch v-if="searchOpen" @close="closeSearch" />
    <KeyboardHelp v-if="helpOpen" @close="closeHelp" />
  </div>
</template>
