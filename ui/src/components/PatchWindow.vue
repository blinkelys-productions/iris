<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { inApp } from '../stores/backend'
import ShareDialog from './ShareDialog.vue'
import {
  applyBulkEdit,
  fieldLabels,
  numberedName,
  rangeBetween,
  type BulkEdit,
  type BulkField,
} from '../stores/bulkEdit'
import { exportMvr, exportPatchPdf, importGdtf, importMvr } from '../stores/interchange'
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
const sharing = ref(false)

type SortKey = Column | null
const sort = reactive({ key: null as SortKey, dir: 1 as 1 | -1 })
const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })

const byPatch = (a: FixtureInstance, b: FixtureInstance) =>
  a.universe - b.universe || a.startAddress - b.startAddress || a.number - b.number

function sortValue(fixture: FixtureInstance, key: Column): number | string {
  switch (key) {
    case 'number': return fixture.number
    case 'name': return fixture.name
    case 'type': return typeLabel(fixture)
    case 'mode': return modeLabel(fixture)
    case 'universe':
    case 'address': return 0
    case 'channels': return channelCount(fixture)
  }
}

const rows = computed(() => {
  const list = [...patch.fixtures]
  const key = sort.key
  if (!key) return list.sort(byPatch)
  return list.sort((a, b) => {
    const x = sortValue(a, key)
    const y = sortValue(b, key)
    const order = key === 'address' ? byPatch(a, b) : typeof x === 'number' ? x - (y as number) : collator.compare(x, y as string)
    return (order || byPatch(a, b) * sort.dir) * sort.dir
  })
})

/** Click cycles ascending, descending, then back to patch order. */
function sortBy(key: Column) {
  if (sort.key !== key) {
    sort.key = key
    sort.dir = 1
  } else if (sort.dir === 1) {
    sort.dir = -1
  } else {
    sort.key = null
    sort.dir = 1
  }
}

const ariaSort = (key: Column) => (sort.key !== key ? 'none' : sort.dir === 1 ? 'ascending' : 'descending')
const arrow = (key: Column) => (sort.key !== key ? '' : sort.dir === 1 ? '▲' : '▼')
const draftErrors = computed(() => (draft.value ? validateFixture(draft.value) : []))
const draftDefinition = computed(() =>
  draft.value ? findDefinition(draft.value.definitionId) : undefined,
)
const selectedFixture = computed(() =>
  selected.value.length === 1 ? patch.fixtures.find((f) => f.id === selected.value[0]) : undefined,
)

function startAdd() {
  const definition = fixtureLibrary[0]
  if (!definition) {
    patch.error = 'Import a GDTF fixture profile before adding fixtures.'
    return
  }
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
        name: numberedName(current.name, i),
        startAddress: address,
      })
      address += channels
    }
  }
  if (await saveFixtures(batch)) draft.value = null
}

type Column = BulkField | 'channels'
const column = ref<Column>('name')
const anchor = ref<string | null>(null)
const dragging = ref(false)
let dragBase: string[] = []

const rowOrder = computed(() => rows.value.map((f) => f.id))

const isMac = /Mac/i.test(navigator.platform)

function onCellDown(id: string, col: Column, event: MouseEvent) {
  if (event.button !== 0) return
  // Ctrl-click is a right click on macOS.
  if (isMac && event.ctrlKey) return
  if (col !== column.value) {
    column.value = col
    selected.value = []
    anchor.value = null
  }
  const additive = event.metaKey || event.ctrlKey
  if (event.shiftKey && anchor.value) {
    const range = rangeBetween(rowOrder.value, anchor.value, id)
    selected.value = additive ? [...new Set([...selected.value, ...range])] : range
    return
  }
  if (additive) {
    selected.value = selected.value.includes(id)
      ? selected.value.filter((item) => item !== id)
      : [...selected.value, id]
    anchor.value = id
    return
  }
  selected.value = [id]
  anchor.value = id
  dragBase = []
  dragging.value = true
}

function onCellEnter(id: string, col: Column) {
  if (col !== column.value) return
  if (!dragging.value || !anchor.value) return
  selected.value = [...new Set([...dragBase, ...rangeBetween(rowOrder.value, anchor.value, id)])]
}

const stopDragging = () => {
  dragging.value = false
}

function onKey(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null
  if (target && ['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName)) return
  if (draft.value || sharing.value) return
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
    event.preventDefault()
    selected.value = [...rowOrder.value]
  } else if (event.key === 'Escape') {
    if (menu.open) menu.open = false
    else selected.value = []
  } else if ((event.key === 'Delete' || event.key === 'Backspace') && selected.value.length) {
    event.preventDefault()
    void deleteSelected()
  }
}

onMounted(() => {
  window.addEventListener('mouseup', stopDragging)
  window.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => {
  window.removeEventListener('mouseup', stopDragging)
  window.removeEventListener('keydown', onKey)
})

const menu = reactive({
  open: false,
  x: 0,
  y: 0,
  edit: { field: 'name', text: '', definitionId: '', modeName: '', sequence: false } as BulkEdit,
})

const selectedFixtures = computed(() => rows.value.filter((f) => selected.value.includes(f.id)))

const menuModes = computed(() => {
  const names = new Set<string>()
  const definitions =
    menu.edit.field === 'type'
      ? [findDefinition(menu.edit.definitionId)]
      : selectedFixtures.value.map((f) => findDefinition(f.definitionId))
  for (const definition of definitions) definition?.modes.forEach((m) => names.add(m.name))
  return [...names]
})

const menuPreview = computed(() => applyBulkEdit(selectedFixtures.value, menu.edit))
const menuErrors = computed(() => {
  const edit = menu.edit
  const textual = ['number', 'universe', 'address'].includes(edit.field)
  if (textual && !Number.isFinite(Number(edit.text))) return ['Enter a number.']
  if (edit.field === 'name' && !edit.text.trim()) return ['Name is required.']
  if ((edit.field === 'type' || edit.field === 'mode') && menu.edit.modeName === '' && menuModes.value.length) {
    return ['Choose a mode.']
  }
  const errors = new Set<string>()
  for (const fixture of menuPreview.value) validateFixture(fixture).forEach((e) => errors.add(e))
  return [...errors]
})

function openMenu(fixture: FixtureInstance, field: BulkField, event: MouseEvent) {
  if (column.value !== field || !selected.value.includes(fixture.id)) {
    column.value = field
    selected.value = [fixture.id]
    anchor.value = fixture.id
  }
  const first = selectedFixtures.value[0] ?? fixture
  const text = { number: first.number, universe: first.universe, address: first.startAddress, name: first.name }
  menu.edit = {
    field,
    text: String(text[field as keyof typeof text] ?? ''),
    definitionId: first.definitionId,
    modeName: findMode(first.definitionId, first.modeId)?.name ?? '',
    sequence: false,
  }
  patch.error = ''
  menu.x = Math.min(event.clientX, window.innerWidth - 280)
  menu.y = Math.min(event.clientY, window.innerHeight - 260)
  menu.open = true
}

function onMenuTypeChange() {
  menu.edit.modeName = findDefinition(menu.edit.definitionId)?.modes[0]?.name ?? ''
}

async function applyMenu() {
  if (menuErrors.value.length) return
  try {
    if (await saveFixtures(menuPreview.value)) menu.open = false
  } catch (error) {
    patch.error = String(error)
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
  return conflict && 'text-red-300 [&>td:first-child]:shadow-[inset_2px_0_0_var(--color-red-500)]'
}

function cellClass(fixture: FixtureInstance, col: Column) {
  const picked = column.value === col && selected.value.includes(fixture.id)
  return picked ? 'bg-accent/20 outline outline-1 -outline-offset-1 outline-accent/60' : 'hover:bg-zinc-800/60'
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
        <table class="w-full select-none border-collapse text-sm">
          <thead class="sticky top-0 bg-zinc-900">
            <tr class="border-b border-zinc-800">
              <th :class="[th, 'cursor-pointer hover:text-zinc-100', 'w-16']" :aria-sort="ariaSort('number')" @click="sortBy('number')">FID <span class="text-accent">{{ arrow('number') }}</span></th>
              <th :class="[th, 'cursor-pointer hover:text-zinc-100']" :aria-sort="ariaSort('name')" @click="sortBy('name')">Name <span class="text-accent">{{ arrow('name') }}</span></th>
              <th :class="[th, 'cursor-pointer hover:text-zinc-100']" :aria-sort="ariaSort('type')" @click="sortBy('type')">Fixture Type <span class="text-accent">{{ arrow('type') }}</span></th>
              <th :class="[th, 'cursor-pointer hover:text-zinc-100']" :aria-sort="ariaSort('mode')" @click="sortBy('mode')">Mode <span class="text-accent">{{ arrow('mode') }}</span></th>
              <th :class="[th, 'cursor-pointer hover:text-zinc-100', 'w-24']" :aria-sort="ariaSort('address')" @click="sortBy('address')">Patch <span class="text-accent">{{ arrow('address') }}</span></th>
              <th :class="[th, 'cursor-pointer hover:text-zinc-100', 'w-14']" :aria-sort="ariaSort('channels')" @click="sortBy('channels')">Ch <span class="text-accent">{{ arrow('channels') }}</span></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="fixture in rows"
              :key="fixture.id"
              class="cursor-default border-b border-zinc-900"
              :class="rowClass(fixture)"
              @dblclick="startEdit(fixture)"
            >
              <td :class="[td, 'tabular-nums text-zinc-400', cellClass(fixture, 'number')]" @mousedown="onCellDown(fixture.id, 'number', $event)" @mouseenter="onCellEnter(fixture.id, 'number')" @contextmenu.prevent="openMenu(fixture, 'number', $event)">{{ fixture.number }}</td>
              <td :class="[td, cellClass(fixture, 'name')]" @mousedown="onCellDown(fixture.id, 'name', $event)" @mouseenter="onCellEnter(fixture.id, 'name')" @contextmenu.prevent="openMenu(fixture, 'name', $event)">{{ fixture.name }}</td>
              <td :class="[td, cellClass(fixture, 'type')]" @mousedown="onCellDown(fixture.id, 'type', $event)" @mouseenter="onCellEnter(fixture.id, 'type')" @contextmenu.prevent="openMenu(fixture, 'type', $event)">{{ typeLabel(fixture) }}</td>
              <td :class="[td, cellClass(fixture, 'mode')]" @mousedown="onCellDown(fixture.id, 'mode', $event)" @mouseenter="onCellEnter(fixture.id, 'mode')" @contextmenu.prevent="openMenu(fixture, 'mode', $event)">{{ modeLabel(fixture) }}</td>
              <td :class="[td, 'tabular-nums', cellClass(fixture, 'address')]" @mousedown="onCellDown(fixture.id, 'address', $event)" @mouseenter="onCellEnter(fixture.id, 'address')" @contextmenu.prevent="openMenu(fixture, 'address', $event)">{{ patchLabel(fixture) }}</td>
              <td :class="[td, 'tabular-nums text-zinc-400', cellClass(fixture, 'channels')]" @mousedown="onCellDown(fixture.id, 'channels', $event)" @mouseenter="onCellEnter(fixture.id, 'channels')">{{ channelCount(fixture) }}</td>
            </tr>
            <tr class="cursor-pointer text-zinc-500 hover:bg-zinc-800/60 hover:text-zinc-200" @click="startAdd">
              <td :class="td"></td>
              <td :class="td" colspan="5">+ New Fixture</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-if="patch.notice && !draft" class="mx-1 mb-1 rounded-md border border-zinc-700 bg-zinc-800/60 px-3 py-2 text-xs text-zinc-300">{{ patch.notice }}</div>
    <div v-if="patch.error && !draft" class="mx-1 mb-1 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-red-300" role="alert">{{ patch.error }}</div>

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
      <button :class="tool" :disabled="!rows.length" @click="selected = [...rowOrder]">Select column</button>
      <button :class="tool" :disabled="!selected.length" @click="selected = []">Clear ({{ selected.length }})</button>
      <button :class="tool" disabled>Cut</button>
      <button :class="tool" disabled>Copy</button>
      <button :class="tool" disabled>Paste</button>
      <span class="mx-1 h-4 w-px bg-zinc-700"></span>
      <button :class="tool" disabled>Patch</button>
      <button :class="tool" :disabled="!inApp" @click="importGdtf">Import GDTF</button>
      <button :class="tool" :disabled="!inApp" @click="sharing = true">GDTF Share</button>
      <button :class="tool" :disabled="!inApp" @click="importMvr">Import MVR</button>
      <button :class="tool" :disabled="!inApp || !patch.fixtures.length" @click="exportMvr">Export MVR</button>
      <button :class="tool" :disabled="!inApp || !patch.fixtures.length" @click="exportPatchPdf">Export PDF</button>
      <button :class="tool" disabled>Create Multipatch</button>
      <span class="ml-auto"></span>
      <button :class="tool" :disabled="!rows.length" @click="moveSelection(-1)">Select up</button>
      <button :class="tool" :disabled="!rows.length" @click="moveSelection(1)">Select down</button>
    </div>

    <div v-if="menu.open" class="fixed inset-0 z-30" @mousedown.self="menu.open = false" @contextmenu.prevent.self="menu.open = false">
      <form
        class="absolute grid w-64 gap-2 rounded-lg border border-zinc-700 bg-zinc-900 p-3 text-xs shadow-2xl"
        :style="{ left: menu.x + 'px', top: menu.y + 'px' }"
        @submit.prevent="applyMenu"
      >
        <div class="font-semibold text-zinc-100">
          {{ fieldLabels[menu.edit.field] }} · {{ selectedFixtures.length }} fixture{{ selectedFixtures.length === 1 ? '' : 's' }}
        </div>
        <select v-if="menu.edit.field === 'type'" v-model="menu.edit.definitionId" class="field" @change="onMenuTypeChange">
          <option v-for="definition in fixtureLibrary" :key="definition.id" :value="definition.id">
            {{ definition.manufacturer }} {{ definition.name }}
          </option>
        </select>
        <select v-if="menu.edit.field === 'type' || menu.edit.field === 'mode'" v-model="menu.edit.modeName" class="field">
          <option v-for="name in menuModes" :key="name" :value="name">{{ name }}</option>
        </select>
        <input
          v-else
          v-model="menu.edit.text"
          class="field"
          :type="menu.edit.field === 'name' ? 'text' : 'number'"
          autofocus
          @focus="($event.target as HTMLInputElement).select()"
        />
        <label v-if="['number', 'address'].includes(menu.edit.field) && selectedFixtures.length > 1" class="flex items-center gap-2 text-zinc-400">
          <input v-model="menu.edit.sequence" type="checkbox" />
          {{ menu.edit.field === 'number' ? 'Count up from this ID' : 'Pack one after another' }}
        </label>
        <ul v-if="menuErrors.length" class="list-disc pl-4 text-red-300">
          <li v-for="error in menuErrors.slice(0, 3)" :key="error">{{ error }}</li>
        </ul>
        <div v-if="patch.error" class="text-red-300" role="alert">{{ patch.error }}</div>
        <div class="flex items-center gap-1">
          <button type="button" class="btn !py-1 text-xs hover:!border-red-500/60" @click="menu.open = false; deleteSelected()">Delete</button>
          <span class="ml-auto"></span>
          <button type="button" class="btn !py-1 text-xs" @click="menu.open = false">Cancel</button>
          <button class="btn btn-primary !py-1 text-xs" :disabled="!!menuErrors.length">Apply</button>
        </div>
      </form>
    </div>

    <ShareDialog v-if="sharing" @close="sharing = false" />

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
