<script setup lang="ts">
import { addView, selectView, workspace } from '../stores/workspace'
</script>

<template>
  <nav class="views flex flex-col overflow-y-auto overscroll-contain bg-zinc-900" aria-label="Views">
    <button
      v-for="(view, index) in workspace.views"
      :key="index"
      class="relative h-16 shrink-0 border-b border-zinc-950 px-1.5 py-1 text-left text-xs font-medium transition-colors"
      :class="
        workspace.activeView === index
          ? 'bg-zinc-700/70 text-accent'
          : 'bg-zinc-900 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200'
      "
      :aria-label="`View ${index + 1}`"
      :aria-current="workspace.activeView === index"
      @click="selectView(index)"
    >
      {{ index + 1 }}
      <span v-if="view.windows.length" class="absolute bottom-1.5 left-1.5 size-1.5 rounded-full bg-accent"></span>
    </button>
    <button
      class="grid h-10 shrink-0 place-items-center text-lg text-zinc-500 hover:bg-zinc-800 hover:text-zinc-100"
      aria-label="Add view"
      title="Add view"
      @click="addView"
    >
      +
    </button>
  </nav>
</template>

<style scoped>
.views {
  scrollbar-width: none;
}
.views::-webkit-scrollbar {
  display: none;
}
</style>
