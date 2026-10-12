<script setup lang="ts">
import { computed, ref } from 'vue'
import Icon from './Icon.vue'
import { channelCount, DMX_SLOTS, patch } from '../stores/patch'
import { menuEntries, titleFor, type WindowKind } from '../stores/workspace'

const props = defineProps<{ kind: WindowKind; columns: number; rows: number }>()

const isPool = computed(
  () => props.kind === 'groups' || props.kind === 'fixtures' || props.kind.startsWith('preset'),
)
const poolTitle = computed(() => titleFor(props.kind).replace(/ Presets$/, ''))
const poolIcon = menuEntries.find((entry) => entry.kind === props.kind)!.icon
const slots = computed(() => Array.from({ length: Math.max(0, props.columns * props.rows - 1) }, (_, i) => i + 1))

const universe = ref(1)
const owners = computed(() => {
  const map = new Map<number, string>()
  for (const fixture of patch.fixtures) {
    if (fixture.universe !== universe.value) continue
    for (let offset = 0; offset < channelCount(fixture); offset++) map.set(fixture.startAddress + offset, fixture.name)
  }
  return map
})
</script>

<template>
  <div
    v-if="isPool"
    class="grid h-full min-h-0 min-w-0 gap-px overflow-hidden bg-zinc-800 p-px"
    :style="{
      gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
      gridTemplateRows: `repeat(${rows}, minmax(0, 1fr))`,
    }"
  >
    <div
      data-drag-handle
      class="relative flex aspect-square min-w-0 cursor-grab flex-col items-center justify-center overflow-hidden bg-cyan-800 px-1 text-center text-[11px] font-semibold leading-tight text-cyan-50 hover:bg-cyan-700"
    >
      <Icon :name="poolIcon" class="mb-0.5 size-4 shrink-0 text-cyan-100" />
      <span class="line-clamp-2">{{ poolTitle }}</span>
    </div>

    <div
      v-for="slot in slots"
      :key="slot"
      class="flex aspect-square min-w-0 flex-col overflow-hidden bg-zinc-950 px-1 py-0.5 text-[10px] leading-tight text-zinc-500"
      :class="kind === 'fixtures' && patch.fixtures[slot - 1] ? 'text-zinc-200' : ''"
    >
      <span class="shrink-0 text-[9px] text-zinc-500">{{ slot }}</span>
      <template v-if="kind === 'fixtures' && patch.fixtures[slot - 1]">
        <span class="truncate text-zinc-300">{{ patch.fixtures[slot - 1]!.number }}</span>
        <span class="truncate text-[10px] text-zinc-100">{{ patch.fixtures[slot - 1]!.name }}</span>
      </template>
    </div>
  </div>

  <div v-else class="h-full overflow-auto p-1">
    <template v-if="kind === 'dmxChart'">
      <div class="mb-1 flex items-center gap-2 text-xs text-zinc-400">
        <label for="dmx-universe">Universe</label>
        <input id="dmx-universe" v-model.number="universe" class="field !w-16 !py-0.5" type="number" min="1" />
      </div>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(30px,1fr))] gap-px">
        <div
          v-for="address in DMX_SLOTS"
          :key="address"
          class="grid h-6 place-items-center text-[10px]"
          :class="owners.has(address) ? 'bg-accent/25 text-accent' : 'bg-zinc-950 text-zinc-600'"
          :title="owners.get(address) ?? 'Free'"
        >
          {{ address }}
        </div>
      </div>
    </template>

    <p v-else class="p-3 text-zinc-500">This module is not implemented yet.</p>
  </div>
</template>
