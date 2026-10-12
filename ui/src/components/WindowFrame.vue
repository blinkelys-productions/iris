<script setup lang="ts">
import { computed, ref } from 'vue'
import Icon from './Icon.vue'
import {
  currentView,
  focusWindow,
  menuEntries,
  moveWindow,
  openModuleSettings,
  resizeWindow,
  titleFor,
  type PlacedWindow,
} from '../stores/workspace'

const props = defineProps<{ win: PlacedWindow }>()
const icon = menuEntries.find((entry) => entry.kind === props.win.kind)!.icon
const isPool = computed(
  () => props.win.kind === 'groups' || props.win.kind === 'fixtures' || props.win.kind.startsWith('preset'),
)

const DRAG_THRESHOLD = 4
const dragStart = ref<{
  pointerId: number
  x: number
  y: number
  col: number
  row: number
  colStep: number
  rowStep: number
  moved: boolean
} | null>(null)

function gridSteps(from: HTMLElement) {
  const grid = from.closest('main')!
  const style = getComputedStyle(grid)
  return {
    colStep: Number.parseFloat(style.gridTemplateColumns.split(' ')[0]!) + Number.parseFloat(style.columnGap),
    rowStep: Number.parseFloat(style.gridTemplateRows.split(' ')[0]!) + Number.parseFloat(style.rowGap),
  }
}

function startDrag(event: PointerEvent) {
  focusWindow(props.win.id)
  if (event.button !== 0 || !(event.target as HTMLElement).closest('[data-drag-handle]')) return
  const section = event.currentTarget as HTMLElement
  section.setPointerCapture(event.pointerId)
  dragStart.value = {
    pointerId: event.pointerId,
    x: event.clientX,
    y: event.clientY,
    col: props.win.col,
    row: props.win.row,
    moved: false,
    ...gridSteps(section),
  }
}

function updateDrag(event: PointerEvent) {
  const start = dragStart.value
  if (!start || start.pointerId !== event.pointerId) return
  const dx = event.clientX - start.x
  const dy = event.clientY - start.y
  if (!start.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) return
  start.moved = true
  moveWindow(props.win.id, start.col + Math.round(dx / start.colStep), start.row + Math.round(dy / start.rowStep))
}

function endDrag(event: PointerEvent) {
  const start = dragStart.value
  if (!start || start.pointerId !== event.pointerId) return
  dragStart.value = null
  if (!start.moved && event.type === 'pointerup') openModuleSettings(props.win.id)
}

const resizeStart = ref<{
  pointerId: number
  x: number
  y: number
  w: number
  h: number
  colStep: number
  rowStep: number
} | null>(null)

function startResize(event: PointerEvent) {
  const handle = event.currentTarget as HTMLElement

  event.preventDefault()
  event.stopPropagation()
  focusWindow(props.win.id)
  handle.setPointerCapture(event.pointerId)

  resizeStart.value = {
    pointerId: event.pointerId,
    x: event.clientX,
    y: event.clientY,
    w: props.win.w,
    h: props.win.h,
    ...gridSteps(handle),
  }
}

function updateResize(event: PointerEvent) {
  const start = resizeStart.value
  if (!start || start.pointerId !== event.pointerId) return
  const w = start.w + Math.round((event.clientX - start.x) / start.colStep)
  const h = start.h + Math.round((event.clientY - start.y) / start.rowStep)
  resizeWindow(props.win.id, w, h)
}

function stopResize(event: PointerEvent) {
  if (resizeStart.value?.pointerId === event.pointerId) resizeStart.value = null
}

function resizeWithKeyboard(event: KeyboardEvent) {
  const changes: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  }
  const change = changes[event.key]
  if (!change) return
  event.preventDefault()
  resizeWindow(props.win.id, props.win.w + change[0], props.win.h + change[1])
}
</script>

<template>
  <section
    class="relative flex min-h-0 min-w-0 flex-col overflow-hidden border bg-zinc-900"
    :class="[
      isPool ? 'rounded-sm' : 'rounded-xl',
      currentView.focused === props.win.id ? 'border-accent/60' : 'border-zinc-800',
    ]"
    :style="{ gridArea: `${win.row + 1} / ${win.col + 1} / span ${win.h} / span ${win.w}` }"
    @pointerdown="startDrag"
    @pointermove="updateDrag"
    @pointerup="endDrag"
    @pointercancel="endDrag"
  >
    <header v-if="!isPool" data-drag-handle class="flex h-7 shrink-0 cursor-grab items-center gap-2 border-b border-zinc-800 bg-zinc-800/60 px-2.5 hover:bg-zinc-800">
      <Icon :name="icon" class="text-zinc-400" />
      <span class="flex-1 truncate text-sm font-medium text-zinc-100">{{ titleFor(props.win.kind) }}</span>
    </header>
    <div class="min-h-0 min-w-0 flex-1">
      <slot />
    </div>
    <button
      class="absolute bottom-0 right-0 z-10 grid size-4 touch-none cursor-se-resize place-items-center rounded-tl bg-zinc-800/90 text-zinc-400 hover:text-accent"
      :aria-label="`Resize ${titleFor(props.win.kind)}`"
      :title="`Resize ${titleFor(props.win.kind)} (${props.win.w} × ${props.win.h} grid)`"
      @pointerdown="startResize"
      @pointermove="updateResize"
      @pointerup="stopResize"
      @pointercancel="stopResize"
      @keydown="resizeWithKeyboard"
    >
      <span class="size-2 border-b-2 border-r-2 border-current"></span>
    </button>
  </section>
</template>
