<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { openShowDialog, type ShowDialogPage } from '../stores/shows'
import {
  menuPages,
  openWindow,
  selectView,
  workspace,
  type MenuPage,
  type WindowKind,
} from '../stores/workspace'

interface Tile {
  label: string
  window?: WindowKind
  view?: number
  dialog?: ShowDialogPage
}

const soon = (...labels: string[]): Tile[] => labels.map((label) => ({ label }))

const pages = computed<Record<MenuPage, Tile[]>>(() => ({
  menu: [
    { label: 'Patch', window: 'patch' },
    ...soon('Live Patch', 'DMX', 'Network', 'Settings'),
    { label: 'Show', dialog: 'show' },
    { label: 'Backup', dialog: 'backup' },
    { label: 'Quick Save', dialog: 'quicksave' },
  ],
  masters: soon('Grand Master', 'Speed Master 1', 'Speed Master 2', 'Rate Master', 'Playback Master', 'Highlight'),
  screens: Array.from({ length: workspace.views.length }, (_, i) => ({ label: `View ${i + 1}`, view: i })),
  console: soon('Desk Lights & Color Theme', 'Connector Configuration', 'In & Out', 'Show Creator', 'Preferences and Timings'),
  help: soon('Manual', 'Keyboard Shortcuts', 'About Iris'),
}))

const tiles = computed(() => pages.value[workspace.menuPage])
const title = computed(() => menuPages.find((p) => p.id === workspace.menuPage)?.title ?? 'Menu')

function activate(tile: Tile) {
  if (tile.window) openWindow(tile.window)
  else if (tile.view !== undefined) selectView(tile.view)
  else if (tile.dialog) openShowDialog(tile.dialog)
}

function isEnabled(tile: Tile) {
  return tile.window !== undefined || tile.view !== undefined || tile.dialog !== undefined
}

function close() {
  workspace.menuOpen = false
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    close()
    workspace.powerOpen = false
  }
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div v-if="workspace.menuOpen" class="absolute inset-0 z-10 grid place-items-center bg-black/50 backdrop-blur-[2px]" @click.self="close">
    <section class="w-[400px] max-w-full overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl" role="dialog" :aria-label="title">
      <header class="flex items-center justify-between border-b border-zinc-800 py-2 pl-4 pr-2">
        <div>
          <h2 class="text-sm font-semibold text-zinc-100">{{ title }} [Iris]</h2>
          <small class="text-[11px] text-zinc-500">Software Version 0.1.0</small>
        </div>
        <button class="grid size-7 place-items-center rounded-md text-xl leading-none text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100" aria-label="Close" @click="close">×</button>
      </header>
      <div class="grid grid-cols-3 gap-1.5 p-3">
        <button
          v-for="tile in tiles"
          :key="tile.label"
          class="h-10 rounded-lg border px-2 text-xs leading-tight transition-colors disabled:opacity-40"
          :class="
            tile.view !== undefined && tile.view === workspace.activeView
              ? 'border-accent bg-accent/10 text-accent'
              : 'border-zinc-700 bg-zinc-800 text-zinc-100 enabled:hover:bg-zinc-700'
          "
          :disabled="!isEnabled(tile)"
          @click="activate(tile)"
        >
          {{ tile.label }}
        </button>
      </div>
    </section>
  </div>
</template>
