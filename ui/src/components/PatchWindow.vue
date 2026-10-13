<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  channelCount,
  conflictedIds,
  conflicts,
  findDefinition,
  findMode,
  fixtureLibrary,
  newFixtureId,
  nextFixtureNumber,
  patch,
  removeFixtures,
  saveFixtures,
  suggestAddress,
  validateFixture,
  type FixtureInstance,
} from '../stores/patch'

const subTabs = [
  { label: 'Patch', available: true },
  { label: 'Fixture Types', available: false },
  { label: 'Attribute Definitions', available: false },
  { label: 'Parameter List', available: false },
  { label: 'DMX Universes', available: false },
  { label: 'Stages', available: false },
  { label: 'DMX Curves', available: false },
]

const selected = ref<string[]>([])
const draft = ref<FixtureInstance | null>(null)
const isNew = ref(false)
const quantity = ref(1)

const rows = computed(() =>
  [...patch.fixtures].sort(
    (a, b) => a.universe - b.universe || a.startAddress - b.startAddress || a.number - b.number,
  ),
)
const draftErrors = computed(() => (draft.value ? validateFixture(draft.value) : []))
const draftDefinition = computed(() =>
  draft.value ? findDefinition(draft.value.definitionId) : undefined,
)
const selectedFixture = computed(() =>
  selected.value.length === 1 ? patch.fixtures.find((f) => f.id === selected.value[0]) : undefined,
)

function startAdd() {
  const definition = fixtureLibrary[0]!
  const mode = definition.modes[0]!
  const universe = rows.value.at(-1)?.universe ?? 1
  isNew.value = true
  quantity.value = 1
  draft.value = {
    id: newFixtureId(),
    number: nextFixtureNumber(),
    name: definition.name,
    definitionId: definition.id,
    modeId: mode.id,
    universe,
    startAddress: suggestAddress(universe, mode.channelCount) ?? 1,
  }
}

function startEdit(fixture: FixtureInstance | undefined = selectedFixture.value) {
  if (!fixture) return
  isNew.value = false
  draft.value = { ...fixture }
}

function onDefinitionChange() {
  const current = draft.value
  const definition = draftDefinition.value
  if (!current || !definition) return
  current.modeId = definition.modes[0]!.id
}

function fitAddress() {
  const current = draft.value
  if (!current) return
  const address = suggestAddress(
    current.universe,
    channelCount(current),
    isNew.value ? undefined : current.id,
  )
  if (address !== null) current.startAddress = address
}

async function commit() {
  const current = draft.value
  if (!current || draftErrors.value.length) return
  const batch: FixtureInstance[] = []
  if (!isNew.value) {
    batch.push(current)
  } else {
    const channels = channelCount(current)
    const count = Math.max(1, Math.floor(quantity.value))
    let address = current.startAddress
    for (let i = 0; i < count; i++) {
      batch.push({
        ...current,
        id: i === 0 ? current.id : newFixtureId(),
        number: current.number + i,
        name: count > 1 ? `${current.name} ${i + 1}` : current.name,
        startAddress: address,
      })
      address += channels
    }
  }
  if (await saveFixtures(batch)) draft.value = null
}

function toggle(id: string, event: MouseEvent) {
  if (event.metaKey || event.ctrlKey || event.shiftKey) {
    selected.value = selected.value.includes(id)
      ? selected.value.filter((item) => item !== id)
      : [...selected.value, id]
  } else {
    selected.value = [id]
  }
}

async function deleteSelected() {
  if (await removeFixtures(selected.value)) selected.value = []
}

function moveSelection(delta: number) {
  if (!rows.value.length) return
  const current = rows.value.findIndex((f) => f.id === selected.value[0])
  const next = Math.min(rows.value.length - 1, Math.max(0, (current === -1 ? (delta > 0 ? -1 : rows.value.length) : current) + delta))
  selected.value = [rows.value[next]!.id]
}

function patchLabel(fixture: FixtureInstance) {
  return `${fixture.universe}.${String(fixture.startAddress).padStart(3, '0')}`
}

function typeLabel(fixture: FixtureInstance) {
  return findDefinition(fixture.definitionId)?.name ?? 'Unknown'
}

function modeLabel(fixture: FixtureInstance) {
  const mode = findMode(fixture.definitionId, fixture.modeId)
  return mode ? mode.name : ''
}

function rowClass(fixture: FixtureInstance) {
  const conflict = conflictedIds.value.has(fixture.id)
  if (selected.value.includes(fixture.id)) {
    return 'bg-accent/10 [&>td:first-child]:shadow-[inset_2px_0_0_var(--color-accent)]'
  }
  return [
    'hover:bg-zinc-800/60',
    conflict && 'text-red-300 [&>td:first-child]:shadow-[inset_2px_0_0_var(--color-red-500)]',
  ]
}

const th = 'px-3 py-2 text-left text-[11px] font-medium uppercase tracking-wide text-zinc-400'
const td = 'px-3 py-1.5'
const tool = 'btn !py-1 text-xs'
</script>

<template>
  <div class="relative flex h-full flex-col">
    <div class="flex min-h-0 flex-1 gap-1 p-1">
      <nav class="flex w-32 shrink-0 flex-col gap-0.5 overflow-y-auto" aria-label="Patch sections">
        <button
          v-for="tab in subTabs"
          :key="tab.label"
          class="rounded-md px-2 py-1 text-left text-xs transition-colors disabled:opacity-40"
          :class="tab.label === 'Patch' ? 'bg-zinc-800 text-accent' : 'text-zinc-400 enabled:hover:bg-zinc-800 enabled:hover:text-zinc-100'"
          :disabled="!tab.available"
        >
          {{ tab.label }}
        </button>
      </nav>

      <div class="min-w-0 flex-1 overflow-auto rounded-lg border border-zinc-800 bg-zinc-950">
        <table class="w-full border-collapse text-sm">
          <thead class="sticky top-0 bg-zinc-900">
            <tr class="border-b border-zinc-800">
              <th :class="[th, 'w-16']">FID</th>
              <th :class="th">Name</th>
              <th :class="th">Fixture Type</th>
              <th :class="th">Mode</th>
              <th :class="[th, 'w-24']">Patch</th>
              <th :class="[th, 'w-14']">Ch</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="fixture in rows"
              :key="fixture.id"
              class="cursor-default border-b border-zinc-900"
              :class="rowClass(fixture)"
              @click="toggle(fixture.id, $event)"
              @dblclick="startEdit(fixture)"
            >
              <td :class="[td, 'tabular-nums text-zinc-400']">{{ fixture.number }}</td>
              <td :class="td">{{ fixture.name }}</td>
              <td :class="td">{{ typeLabel(fixture) }}</td>
              <td :class="td">{{ modeLabel(fixture) }}</td>
              <td :class="[td, 'tabular-nums']">{{ patchLabel(fixture) }}</td>
              <td :class="[td, 'tabular-nums text-zinc-400']">{{ channelCount(fixture) }}</td>
            </tr>
            <tr class="cursor-pointer text-zinc-500 hover:bg-zinc-800/60 hover:text-zinc-200" @click="startAdd">
              <td :class="td"></td>
              <td :class="td" colspan="5">+ New Fixture</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-if="conflicts.length" class="mx-1 mb-1 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-red-300" role="alert">
      <strong>{{ conflicts.length }} conflict{{ conflicts.length === 1 ? '' : 's' }}</strong>
      <ul class="mt-1 list-disc pl-4">
        <li v-for="(conflict, index) in conflicts" :key="index">{{ conflict.message }}</li>
      </ul>
    </div>

    <div class="flex flex-wrap items-center gap-1 border-t border-zinc-800 p-1">
      <button class="btn btn-primary !py-1 text-xs" @click="startAdd">Insert New Fixture</button>
      <button :class="tool" :disabled="!selectedFixture" @click="startEdit()">Edit DMX Mode</button>
      <button :class="[tool, 'hover:!border-red-500/60 hover:!bg-red-500/20']" :disabled="!selected.length" @click="deleteSelected">Delete</button>
      <span class="mx-1 h-4 w-px bg-zinc-700"></span>
      <button :class="tool" disabled>Cut</button>
      <button :class="tool" disabled>Copy</button>
      <button :class="tool" disabled>Paste</button>
      <span class="mx-1 h-4 w-px bg-zinc-700"></span>
      <button :class="tool" disabled>Patch</button>
      <button :class="tool" disabled>Export</button>
      <button :class="tool" disabled>Create Multipatch</button>
      <span class="ml-auto"></span>
      <button :class="tool" :disabled="!rows.length" @click="moveSelection(-1)">Select up</button>
      <button :class="tool" :disabled="!rows.length" @click="moveSelection(1)">Select down</button>
    </div>

    <div v-if="draft" class="absolute inset-0 z-10 grid place-items-center bg-black/50 p-3">
      <form class="grid w-80 max-w-full gap-3 rounded-xl border border-zinc-700 bg-zinc-900 p-4 shadow-2xl" @submit.prevent="commit">
        <h3 class="text-sm font-semibold text-zinc-100">{{ isNew ? 'Insert new fixtures' : 'Edit fixture' }}</h3>
        <label class="grid gap-1 text-xs text-zinc-400">Fixture type
          <select v-model="draft.definitionId" class="field" @change="onDefinitionChange">
            <option v-for="definition in fixtureLibrary" :key="definition.id" :value="definition.id">
              {{ definition.manufacturer }} {{ definition.name }}
            </option>
          </select>
        </label>
        <label class="grid gap-1 text-xs text-zinc-400">Mode
          <select v-model="draft.modeId" class="field">
            <option v-for="mode in draftDefinition?.modes" :key="mode.id" :value="mode.id">
              {{ mode.name }} ({{ mode.channelCount }} ch)
            </option>
          </select>
        </label>
        <label class="grid gap-1 text-xs text-zinc-400">Name <input v-model="draft.name" class="field" /></label>
        <div class="grid grid-cols-2 gap-3">
          <label class="grid gap-1 text-xs text-zinc-400">Fixture ID <input v-model.number="draft.number" class="field" type="number" min="1" /></label>
          <label v-if="isNew" class="grid gap-1 text-xs text-zinc-400">Quantity <input v-model.number="quantity" class="field" type="number" min="1" /></label>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <label class="grid gap-1 text-xs text-zinc-400">Universe <input v-model.number="draft.universe" class="field" type="number" min="1" /></label>
          <label class="grid gap-1 text-xs text-zinc-400">Address
            <input v-model.number="draft.startAddress" class="field" type="number" min="1" :max="512" />
          </label>
        </div>
        <button type="button" class="btn !py-1 text-xs" @click="fitAddress">First free</button>
        <p v-if="patch.error" class="text-xs text-red-400">{{ patch.error }}</p>
        <ul v-if="draftErrors.length" class="list-disc pl-4 text-xs text-red-400">
          <li v-for="error in draftErrors" :key="error">{{ error }}</li>
        </ul>
        <div class="flex justify-end gap-2">
          <button type="button" class="btn" @click="draft = null">Cancel</button>
          <button type="submit" class="btn btn-primary" :disabled="draftErrors.length > 0">
            {{ isNew ? 'Add' : 'Save' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
