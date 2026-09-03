<script setup lang="ts">
import MarkdownIt from "markdown-it";
import { computed } from "vue";

const props = defineProps<{ source: string }>();

const renderer = new MarkdownIt({
  breaks: true,
  html: false,
  linkify: true,
  typographer: false,
});

const rendered = computed(() => renderer.render(props.source));
</script>

<template>
  <!-- eslint-disable-next-line vue/no-v-html -- MarkdownIt has raw HTML disabled and rejects unsafe links. -->
  <div class="rendered-markdown" v-html="rendered"></div>
</template>
