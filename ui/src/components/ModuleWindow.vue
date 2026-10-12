<script setup lang="ts">
import { computed, ref } from 'vue'
import { channelCount, DMX_SLOTS, patch } from '../stores/patch'
import type { WindowKind } from '../stores/workspace'

const props = defineProps<{ kind: WindowKind }>()

const isPool = computed(() => props.kind === 'groups' || props.kind.startsWith('preset'))
const slots = Array.from({ length: 48 }, (_, i) => i + 1)

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
  <div class="h-full overflow-auto p-1">
    <div v-if="isPool" class="grid grid-cols-[repeat(auto-fill,minmax(64px,1fr))] gap-1">
      <div v-for="slot in slots" :key="slot" class="h-14 rounded-md border border-zinc-800 bg-zinc-950 px-1.5 py-1 text-[11px] text-zinc-600">{{ slot }}</div>
    </div>

    <template v-else-if="kind === 'fixtures'">
      <p v-if="!patch.fixtures.length" class="p-3 text-zinc-500">No fixtures patched yet.</p>
      <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(80px,1fr))] gap-1">
        <div v-for="fixture in patch.fixtures" :key="fixture.id" class="h-14 rounded-md border border-zinc-700 bg-zinc-800 px-1.5 py-1 text-xs">
          <div class="text-[11px] text-zinc-400">{{ fixture.number }}</div>
          <div class="truncate text-zinc-100">{{ fixture.name }}</div>
        </div>
      </div>
    </template>

    <template v-else-if="kind === 'dmxChart'">
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
