<script setup lang="ts">
import { onMounted } from "vue";
import { useRoute } from "vue-router";
import AppShell from "@/app/AppShell.vue";
import DocsView from "@/features/docs/DocsView.vue";
import BootScreen from "@/features/session/BootScreen.vue";
import SignInView from "@/features/session/SignInView.vue";
import { useWorkspaceStore } from "@/stores/workspace";

const workspace = useWorkspaceStore();
const route = useRoute();
onMounted(() => void workspace.bootstrap());
</script>

<template>
  <DocsView v-if="route.name === 'docs' && workspace.sessionState !== 'ready'" />
  <BootScreen v-else-if="workspace.sessionState === 'loading'" />
  <SignInView v-else-if="workspace.sessionState === 'idle'" />
  <main v-else-if="workspace.sessionState === 'error'" class="state-page">
    <div class="state-card">
      <div class="brand-glyph" aria-hidden="true">D</div>
      <p class="eyebrow">Service unavailable</p>
      <h1>Devcenter could not resolve your session.</h1>
      <p>{{ workspace.sessionError }}</p>
      <button class="button primary" type="button" @click="workspace.bootstrap">Try again</button>
    </div>
  </main>
  <AppShell v-else />
</template>
