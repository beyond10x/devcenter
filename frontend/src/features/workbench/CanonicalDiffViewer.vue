<script setup lang="ts">
import { ChevronsDownUp, ChevronsUpDown, Paperclip, Search } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import type { DiffHunk, DiffLine, DiffMode, DiffProjection } from "@/api/client";

const props = defineProps<{
  projection: DiffProjection;
  layout: "unified" | "side_by_side";
}>();
const emit = defineEmits<{
  mode: [mode: DiffMode];
  layout: [layout: "unified" | "side_by_side"];
  attach: [hunk: DiffHunk, path: string];
}>();

const query = ref("");
const collapsed = ref(new Set<string>());
const visibleFiles = computed(() => {
  const normalized = query.value.trim().toLocaleLowerCase();
  if (!normalized) return props.projection.files;
  return props.projection.files.filter((file) =>
    (file.new_path ?? file.old_path ?? "").toLocaleLowerCase().includes(normalized),
  );
});

watch(
  () => props.projection.digest,
  () => (collapsed.value = new Set()),
);

function pathFor(file: DiffProjection["files"][number]): string {
  return file.new_path ?? file.old_path ?? "unknown";
}

function toggle(path: string) {
  const next = new Set(collapsed.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  collapsed.value = next;
}

function collapseAll() {
  collapsed.value = new Set(visibleFiles.value.map(pathFor));
}

function expandAll() {
  collapsed.value = new Set();
}

interface SplitDiffRow {
  old?: DiffLine;
  current?: DiffLine;
}

function splitRows(hunk: DiffHunk): SplitDiffRow[] {
  const rows: SplitDiffRow[] = [];
  let deletions: DiffLine[] = [];
  let additions: DiffLine[] = [];
  const flushChanges = () => {
    const length = Math.max(deletions.length, additions.length);
    for (let index = 0; index < length; index += 1) {
      rows.push({ old: deletions[index], current: additions[index] });
    }
    deletions = [];
    additions = [];
  };
  for (const line of hunk.lines) {
    if (line.kind === "deletion") {
      deletions.push(line);
    } else if (line.kind === "addition") {
      additions.push(line);
    } else {
      flushChanges();
      rows.push({ old: line, current: line });
    }
  }
  flushChanges();
  return rows;
}
</script>

<template>
  <section class="canonical-diff" aria-label="Canonical workspace diff">
    <header class="diff-toolbar">
      <div class="diff-summary">
        <strong>{{ projection.files.length }} changed</strong>
        <span class="diff-additions">+{{ projection.additions }}</span>
        <span class="diff-deletions">−{{ projection.deletions }}</span>
        <code :title="projection.digest">{{ projection.digest.slice(0, 12) }}</code>
      </div>
      <label class="diff-search">
        <Search :size="14" /><span class="sr-only">Search changed files</span>
        <input v-model="query" type="search" placeholder="Changed file" />
      </label>
      <div class="segmented-control" aria-label="Diff detail">
        <button
          v-for="mode in ['patch', 'stat', 'files_only'] as const"
          :key="mode"
          type="button"
          :class="{ active: projection.mode === mode }"
          @click="emit('mode', mode)"
        >
          {{ mode.replace("_", " ") }}
        </button>
      </div>
      <div class="segmented-control" aria-label="Diff layout">
        <button
          type="button"
          :class="{ active: layout === 'unified' }"
          @click="emit('layout', 'unified')"
        >
          Unified
        </button>
        <button
          type="button"
          :class="{ active: layout === 'side_by_side' }"
          @click="emit('layout', 'side_by_side')"
        >
          Split
        </button>
      </div>
      <button class="icon-button" type="button" title="Collapse all" @click="collapseAll">
        <ChevronsDownUp :size="16" />
      </button>
      <button class="icon-button" type="button" title="Expand all" @click="expandAll">
        <ChevronsUpDown :size="16" />
      </button>
    </header>

    <div v-if="projection.partial" class="workbench-notice warning-state">
      This projection is explicitly partial. Do not use it as publication evidence.
    </div>
    <div v-if="!visibleFiles.length" class="workbench-empty">
      <strong>{{
        projection.files.length ? "No changed file matches" : "No working changes"
      }}</strong>
      <p>
        {{
          projection.files.length
            ? "Clear the changed-file search."
            : "The working materialization matches its immutable base."
        }}
      </p>
    </div>
    <article v-for="file in visibleFiles" :key="pathFor(file)" class="diff-file">
      <button class="diff-file-header" type="button" @click="toggle(pathFor(file))">
        <span class="status-pill">{{ file.status }}</span>
        <code>{{ pathFor(file) }}</code>
        <span>{{ file.additions == null ? "?" : `+${file.additions}` }}</span>
        <span>{{ file.deletions == null ? "?" : `−${file.deletions}` }}</span>
      </button>
      <div v-if="!collapsed.has(pathFor(file)) && projection.mode === 'patch'" class="diff-hunks">
        <section v-for="hunk in file.hunks" :key="hunk.id" class="diff-hunk">
          <header>
            <code
              >@@ -{{ hunk.old.start }},{{ hunk.old.lines }} +{{ hunk.new.start }},{{
                hunk.new.lines
              }}
              @@</code
            >
            <span>{{ hunk.heading }}</span>
            <button
              class="button quiet small"
              type="button"
              @click="emit('attach', hunk, pathFor(file))"
            >
              <Paperclip :size="13" /> Attach hunk
            </button>
          </header>
          <div v-if="layout === 'unified'" class="diff-lines unified">
            <div
              v-for="(line, index) in hunk.lines"
              :key="`${hunk.id}:${index}`"
              :class="`diff-line ${line.kind}`"
            >
              <span>{{ line.old_line ?? "" }}</span
              ><span>{{ line.new_line ?? "" }}</span
              ><code>{{ line.content }}</code>
            </div>
          </div>
          <div v-else class="diff-lines split">
            <div class="diff-side old-side">
              <div
                v-for="(row, index) in splitRows(hunk)"
                :key="`${hunk.id}:old:${index}`"
                :class="`diff-line ${row.old?.kind ?? 'empty'}`"
              >
                <span>{{ row.old?.old_line ?? "" }}</span
                ><code>{{ row.old?.content ?? "" }}</code>
              </div>
            </div>
            <div class="diff-side new-side">
              <div
                v-for="(row, index) in splitRows(hunk)"
                :key="`${hunk.id}:new:${index}`"
                :class="`diff-line ${row.current?.kind ?? 'empty'}`"
              >
                <span>{{ row.current?.new_line ?? "" }}</span
                ><code>{{ row.current?.content ?? "" }}</code>
              </div>
            </div>
          </div>
        </section>
      </div>
    </article>
  </section>
</template>
