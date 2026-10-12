<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { closeModuleSettings, closeWindow, currentView, titleFor, workspace } from '../stores/workspace'

const win = computed(() => currentView.value.windows.find((item) => item.id === workspace.moduleSettings))

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') closeModuleSettings()
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div v-if="win" class="absolute inset-0 z-10 grid place-items-center bg-black/50 backdrop-blur-[2px]" @click.self="closeModuleSettings">
    <section class="w-[300px] max-w-full overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl" role="dialog" aria-label="Module settings">
      <header class="flex items-center justify-between border-b border-zinc-800 py-2 pl-4 pr-2">
        <div>
          <h2 class="text-sm font-semibold text-zinc-100">{{ titleFor(win.kind) }}</h2>
          <small class="text-[11px] text-zinc-500">Module settings · {{ win.w }} × {{ win.h }}</small>
        </div>
        <button class="grid size-7 place-items-center rounded-md text-xl leading-none text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100" aria-label="Close" @click="closeModuleSettings">×</button>
      </header>
      <div class="grid gap-1.5 p-3">
        <button class="h-10 rounded-lg border border-red-900 bg-red-950/60 px-2 text-xs text-red-200 transition-colors hover:bg-red-900/60" @click="closeWindow(win.id)">
          Delete module
        </button>
      </div>
    </section>
  </div>
</template>
