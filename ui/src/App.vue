<script setup lang="ts">
import { computed } from 'vue'
import BottomPanel from './components/BottomPanel.vue'
import CommandBar from './components/CommandBar.vue'
import MainMenu from './components/MainMenu.vue'
import ModulePicker from './components/ModulePicker.vue'
import ModuleWindow from './components/ModuleWindow.vue'
import PatchWindow from './components/PatchWindow.vue'
import PowerMenu from './components/PowerMenu.vue'
import SideBar from './components/SideBar.vue'
import ViewStrip from './components/ViewStrip.vue'
import WindowFrame from './components/WindowFrame.vue'
import { currentView, GRID_COLS, GRID_ROWS, isCellFree, openPicker, showBottomPanel } from './stores/workspace'

// Recomputed whenever windows change, so occupied cells stop being clickable.
const emptyCells = computed(() => {
  const cells: { col: number; row: number }[] = []
  for (let row = 0; row < GRID_ROWS; row++)
    for (let col = 0; col < GRID_COLS; col++) if (isCellFree(col, row)) cells.push({ col, row })
  return cells
})

const gridStyle = {
  gridTemplateColumns: `repeat(${GRID_COLS}, minmax(0, 1fr))`,
  gridTemplateRows: `repeat(${GRID_ROWS}, minmax(0, 1fr))`,
}
</script>

<template>
  <div class="grid h-full grid-cols-[40px_minmax(0,1fr)_56px] bg-zinc-900" :class="showBottomPanel ? 'grid-rows-[minmax(0,1fr)_auto_auto]' : 'grid-rows-[minmax(0,1fr)_auto]'">
    <SideBar />
    <main class="relative grid h-full min-h-0 min-w-0 overflow-hidden bg-zinc-950 p-1 gap-1" :style="gridStyle">
      <button
        v-for="cell in emptyCells"
        :key="`${cell.col}-${cell.row}`"
        class="group grid place-items-center rounded-md"
        :style="{ gridArea: `${cell.row + 1} / ${cell.col + 1}` }"
        :aria-label="`Add module at column ${cell.col + 1}, row ${cell.row + 1}`"
        @click="openPicker(cell.col, cell.row)"
      >
        <span class="size-1 rounded-full bg-zinc-700 transition-all group-hover:size-2 group-hover:bg-accent"></span>
      </button>
      <WindowFrame v-for="win in currentView.windows" :key="win.id" :win="win">
        <PatchWindow v-if="win.kind === 'patch'" />
        <ModuleWindow v-else :kind="win.kind" />
      </WindowFrame>
      <MainMenu />
      <ModulePicker />
    </main>
    <ViewStrip />
    <CommandBar />
    <BottomPanel v-if="showBottomPanel" />
    <PowerMenu />
  </div>
</template>
