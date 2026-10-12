<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import {
  addWindow,
  canPlaceModule,
  closePicker,
  menuEntries,
  moduleGroups,
  workspace,
  type WindowKind,
} from '../stores/workspace'

const entry = (kind: WindowKind) => menuEntries.find((item) => item.kind === kind)!
const canFitAnything = computed(
  () =>
    !!workspace.picker &&
    moduleGroups.some((group) =>
      group.kinds.some((kind) => entry(kind).available && canPlaceModule(kind, workspace.picker!)),
    ),
)

function canChoose(kind: WindowKind) {
  return entry(kind).available && !!workspace.picker && canPlaceModule(kind, workspace.picker)
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') closePicker()
}

function choose(kind: WindowKind) {
  if (workspace.picker) addWindow(kind, workspace.picker)
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div v-if="workspace.picker" class="absolute inset-0 z-10 grid place-items-center bg-black/50 backdrop-blur-[2px]" @click.self="closePicker">
    <section class="max-h-full w-[400px] max-w-full overflow-y-auto rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl" role="dialog" aria-label="Add module">
      <header class="flex items-center justify-between border-b border-zinc-800 py-2 pl-4 pr-2">
        <div>
          <h2 class="text-sm font-semibold text-zinc-100">Add module</h2>
          <small class="text-[11px] text-zinc-500">Column {{ workspace.picker.col + 1 }}, row {{ workspace.picker.row + 1 }}</small>
        </div>
        <button class="grid size-7 place-items-center rounded-md text-xl leading-none text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100" aria-label="Close" @click="closePicker">×</button>
      </header>
      <div class="grid gap-2 p-3">
        <div v-for="group in moduleGroups" :key="group.title">
          <h3 class="mb-1 text-[11px] font-medium uppercase tracking-wide text-zinc-500">{{ group.title }}</h3>
          <div class="grid grid-cols-3 gap-1.5">
            <button
              v-for="kind in group.kinds"
              :key="kind"
              class="h-10 rounded-lg border border-zinc-700 bg-zinc-800 px-2 text-xs leading-tight text-zinc-100 transition-colors enabled:hover:bg-zinc-700 disabled:opacity-40"
              :disabled="!canChoose(kind)"
              @click="choose(kind)"
            >
              {{ entry(kind).title.replace(' Presets', '') }}
            </button>
          </div>
        </div>
        <p v-if="!canFitAnything" class="text-xs text-zinc-500">
          No module fits at this grid position. Choose an open area with enough space.
        </p>
      </div>
    </section>
  </div>
</template>
