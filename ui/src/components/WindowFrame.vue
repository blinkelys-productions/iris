<script setup lang="ts">
import Icon from './Icon.vue'
import { closeWindow, currentView, focusWindow, menuEntries, titleFor, type PlacedWindow } from '../stores/workspace'

const props = defineProps<{ win: PlacedWindow }>()
const icon = menuEntries.find((entry) => entry.kind === props.win.kind)!.icon
</script>

<template>
  <section
    class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border bg-zinc-900"
    :class="currentView.focused === props.win.id ? 'border-accent/60' : 'border-zinc-800'"
    :style="{ gridArea: `${win.row + 1} / ${win.col + 1} / span ${win.h} / span ${win.w}` }"
    @pointerdown="focusWindow(props.win.id)"
  >
    <header class="flex h-7 shrink-0 items-center gap-2 border-b border-zinc-800 bg-zinc-800/60 pl-2.5 pr-1">
      <Icon :name="icon" class="text-zinc-400" />
      <span class="flex-1 truncate text-sm font-medium text-zinc-100">{{ titleFor(props.win.kind) }}</span>
      <button
        class="grid size-6 place-items-center rounded-md text-lg leading-none text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
        :aria-label="`Close ${titleFor(props.win.kind)}`"
        @click="closeWindow(props.win.id)"
      >
        ×
      </button>
    </header>
    <div class="min-h-0 flex-1">
      <slot />
    </div>
  </section>
</template>
