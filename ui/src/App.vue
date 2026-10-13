<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import BottomPanel from './components/BottomPanel.vue'
import CommandBar from './components/CommandBar.vue'
import MainMenu from './components/MainMenu.vue'
import ModulePicker from './components/ModulePicker.vue'
import ModuleSettings from './components/ModuleSettings.vue'
import ShowDialog from './components/ShowDialog.vue'
import ModuleWindow from './components/ModuleWindow.vue'
import PatchWindow from './components/PatchWindow.vue'
import PowerMenu from './components/PowerMenu.vue'
import SideBar from './components/SideBar.vue'
import ViewStrip from './components/ViewStrip.vue'
import WindowFrame from './components/WindowFrame.vue'
import { bootstrap } from './stores/shows'
import { currentView, GRID_COLS, GRID_ROWS, isCellFree, onPatchPage, openPicker } from './stores/workspace'

// Recomputed whenever windows change, so occupied cells stop being clickable.
const emptyCells = computed(() => {
  const cells: { col: number; row: number }[] = []
  for (let row = 0; row < GRID_ROWS; row++)
    for (let col = 0; col < GRID_COLS; col++) if (isCellFree(col, row)) cells.push({ col, row })
  return cells
})

const gridElement = ref<HTMLElement | null>(null)
const cellSize = ref(0)
const gridStyle = computed(() => ({
  gridTemplateColumns: `repeat(${GRID_COLS}, ${cellSize.value}px)`,
  gridTemplateRows: `repeat(${GRID_ROWS}, ${cellSize.value}px)`,
}))

let gridObserver: ResizeObserver | undefined

onMounted(() => {
  bootstrap()
  gridObserver = new ResizeObserver(([entry]) => {
    if (!entry) return
    const gap = 4
    const width = (entry.contentRect.width - gap * (GRID_COLS - 1)) / GRID_COLS
    const height = (entry.contentRect.height - gap * (GRID_ROWS - 1)) / GRID_ROWS
    cellSize.value = Math.max(0, Math.min(width, height))
  })
  if (gridElement.value) gridObserver.observe(gridElement.value)
})

onBeforeUnmount(() => gridObserver?.disconnect())
</script>

<template>
  <div
    class="grid h-full bg-zinc-900"
    :class="[
      onPatchPage ? 'grid-cols-[40px_minmax(0,1fr)]' : 'grid-cols-[40px_minmax(0,1fr)_56px]',
      onPatchPage ? 'grid-rows-[minmax(0,1fr)_auto]' : 'grid-rows-[minmax(0,1fr)_auto_auto]',
    ]"
  >
    <SideBar />
    <div class="relative h-full min-h-0 min-w-0 bg-zinc-950">
      <main v-if="onPatchPage" class="h-full min-h-0 min-w-0 overflow-hidden">
        <PatchWindow />
      </main>
      <main v-show="!onPatchPage" ref="gridElement" class="relative grid h-full min-h-0 min-w-0 place-content-center overflow-hidden p-1 gap-1" :style="gridStyle">
        <button
          v-for="cell in emptyCells"
          :key="`${cell.col}-${cell.row}`"
          class="group relative rounded-md"
          :style="{ gridArea: `${cell.row + 1} / ${cell.col + 1}` }"
          :aria-label="`Add module at column ${cell.col + 1}, row ${cell.row + 1}`"
          @click="openPicker(cell.col, cell.row)"
        >
          <span class="absolute left-0 top-0 size-1 -translate-x-1/2 -translate-y-1/2 rounded-full bg-zinc-700 transition-all group-hover:size-2 group-hover:bg-accent"></span>
        </button>
        <WindowFrame v-for="win in currentView.windows" :key="win.id" :win="win">
          <ModuleWindow :kind="win.kind" :columns="win.w" :rows="win.h" />
        </WindowFrame>
      </main>
      <MainMenu />
      <ModulePicker />
      <ModuleSettings />
      <ShowDialog />
    </div>
    <ViewStrip v-if="!onPatchPage" />
    <CommandBar />
    <BottomPanel v-if="!onPatchPage" />
    <PowerMenu />
  </div>
</template>
