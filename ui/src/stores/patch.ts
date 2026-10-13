import { computed, reactive } from 'vue'
import { command, inApp } from './backend'

export const DMX_SLOTS = 512

export interface FixtureMode {
  id: string
  name: string
  channelCount: number
}

export interface FixtureDefinition {
  id: string
  manufacturer: string
  name: string
  modes: FixtureMode[]
}

export interface FixtureInstance {
  id: string
  number: number
  name: string
  definitionId: string
  modeId: string
  universe: number
  startAddress: number
}

export interface PatchConflict {
  fixtureIds: string[]
  message: string
}

// Stand-ins for the browser preview; in the app the library comes from imported GDTF profiles.
const placeholderLibrary: FixtureDefinition[] = [
  {
    id: 'generic-dimmer',
    manufacturer: 'Generic',
    name: 'Dimmer',
    modes: [{ id: 'default', name: 'Default', channelCount: 1 }],
  },
  {
    id: 'generic-rgb',
    manufacturer: 'Generic',
    name: 'RGB LED',
    modes: [{ id: 'rgb', name: 'RGB', channelCount: 3 }],
  },
  {
    id: 'generic-rgbw-wash',
    manufacturer: 'Generic',
    name: 'RGBW Wash',
    modes: [
      { id: 'basic', name: 'Basic', channelCount: 5 },
      { id: 'extended', name: 'Extended', channelCount: 9 },
    ],
  },
  {
    id: 'generic-spot',
    manufacturer: 'Generic',
    name: 'Moving Spot',
    modes: [
      { id: 'standard', name: 'Standard', channelCount: 16 },
      { id: 'extended', name: 'Extended', channelCount: 24 },
    ],
  },
]

export const fixtureLibrary: FixtureDefinition[] = reactive(inApp ? [] : placeholderLibrary)

export const patch = reactive({
  fixtures: [] as FixtureInstance[],
  /** True while edits are stored by the backend with the open show. */
  synced: false,
  error: '',
  notice: '',
})

interface WireFixture {
  id: string
  number: number
  name: string
  definition_id: string
  mode_id: string
  universe: number
  start_address: number
}

export interface WireDefinition {
  id: string
  manufacturer: string
  name: string
  modes: { id: string; name: string; channel_count: number }[]
}

interface WirePatch {
  fixture_definitions: WireDefinition[]
  fixtures: WireFixture[]
}

const toWire = (f: FixtureInstance): WireFixture => ({
  id: f.id,
  number: f.number,
  name: f.name,
  definition_id: f.definitionId,
  mode_id: f.modeId,
  universe: f.universe,
  start_address: f.startAddress,
})

const definitionToWire = (d: FixtureDefinition): WireDefinition => ({
  id: d.id,
  manufacturer: d.manufacturer,
  name: d.name,
  modes: d.modes.map((m) => ({ id: m.id, name: m.name, channel_count: m.channelCount })),
})

/** Replaces the local patch with the backend's, adding any definitions the library lacks. */
export function applyWirePatch(wire: WirePatch) {
  for (const d of wire.fixture_definitions) {
    if (findDefinition(d.id)) continue
    fixtureLibrary.push({
      id: d.id,
      manufacturer: d.manufacturer,
      name: d.name,
      modes: d.modes.map((m) => ({ id: m.id, name: m.name, channelCount: m.channel_count })),
    })
  }
  patch.fixtures = wire.fixtures.map((f) => ({
    id: f.id,
    number: f.number,
    name: f.name,
    definitionId: f.definition_id,
    modeId: f.mode_id,
    universe: f.universe,
    startAddress: f.start_address,
  }))
}

export function addToLibrary(d: WireDefinition) {
  const definition: FixtureDefinition = {
    id: d.id,
    manufacturer: d.manufacturer,
    name: d.name,
    modes: d.modes.map((m) => ({ id: m.id, name: m.name, channelCount: m.channel_count })),
  }
  const index = fixtureLibrary.findIndex((existing) => existing.id === d.id)
  if (index === -1) fixtureLibrary.push(definition)
  else fixtureLibrary[index] = definition
}

/** Loads every imported GDTF profile into the library. */
export async function loadProfiles() {
  for (const d of await command<WireDefinition[]>('list_profiles')) addToLibrary(d)
}

export async function importGdtfFile(path: string) {
  addToLibrary(await command<WireDefinition>('import_gdtf', { path }))
}

export async function importMvrFile(path: string) {
  const result = await command<{ patch: WirePatch; fixtures_added: number; warnings: string[] }>(
    'import_mvr',
    { path },
  )
  applyWirePatch(result.patch)
  return result
}

export async function loadPatch() {
  patch.synced = true
  patch.error = ''
  applyWirePatch(await command<WirePatch>('get_patch'))
}

export function clearPatch() {
  patch.synced = false
  patch.fixtures = []
}

export function findDefinition(id: string) {
  return fixtureLibrary.find((definition) => definition.id === id)
}

export function findMode(definitionId: string, modeId: string) {
  return findDefinition(definitionId)?.modes.find((mode) => mode.id === modeId)
}

export function channelCount(fixture: FixtureInstance) {
  return findMode(fixture.definitionId, fixture.modeId)?.channelCount ?? 0
}

export function endAddress(fixture: FixtureInstance) {
  return fixture.startAddress + channelCount(fixture) - 1
}

export function validateFixture(fixture: FixtureInstance): string[] {
  const errors: string[] = []
  if (!Number.isInteger(fixture.number) || fixture.number < 1) {
    errors.push('Fixture ID must be a whole number greater than zero.')
  }
  if (!fixture.name.trim()) errors.push('Name is required.')
  if (!findMode(fixture.definitionId, fixture.modeId)) {
    errors.push('Select a fixture type and mode.')
  }
  if (!Number.isInteger(fixture.universe) || fixture.universe < 1) {
    errors.push('Universe must be a whole number greater than zero.')
  }
  if (
    !Number.isInteger(fixture.startAddress) ||
    fixture.startAddress < 1 ||
    fixture.startAddress > DMX_SLOTS
  ) {
    errors.push(`Address must be between 1 and ${DMX_SLOTS}.`)
  } else if (endAddress(fixture) > DMX_SLOTS) {
    errors.push(
      `Uses channels ${fixture.startAddress}-${endAddress(fixture)}, past the end of the universe (${DMX_SLOTS}).`,
    )
  }
  return errors
}

/** Conflicts across the whole patch, matching the rules in the Rust patch model. */
export function findConflicts(fixtures: FixtureInstance[]): PatchConflict[] {
  const conflicts: PatchConflict[] = []
  const byNumber = new Map<number, FixtureInstance[]>()
  for (const fixture of fixtures) {
    byNumber.set(fixture.number, [...(byNumber.get(fixture.number) ?? []), fixture])
  }
  for (const [number, group] of byNumber) {
    if (group.length > 1) {
      conflicts.push({
        fixtureIds: group.map((fixture) => fixture.id),
        message: `Fixture ID ${number} is assigned more than once.`,
      })
    }
  }

  const placed = fixtures
    .filter((fixture) => validateFixture(fixture).length === 0)
    .sort(
      (a, b) =>
        a.universe - b.universe || a.startAddress - b.startAddress || a.number - b.number,
    )
  for (let i = 0; i < placed.length; i++) {
    for (let j = i + 1; j < placed.length; j++) {
      const a = placed[i]!
      const b = placed[j]!
      if (a.universe !== b.universe) break
      if (b.startAddress > endAddress(a)) break
      conflicts.push({
        fixtureIds: [a.id, b.id],
        message: `${a.name} and ${b.name} overlap in universe ${a.universe}, addresses ${b.startAddress}-${Math.min(endAddress(a), endAddress(b))}.`,
      })
    }
  }
  return conflicts
}

export const conflicts = computed(() => findConflicts(patch.fixtures))

export const conflictedIds = computed(
  () => new Set(conflicts.value.flatMap((conflict) => conflict.fixtureIds)),
)

export function nextFixtureNumber() {
  return patch.fixtures.reduce((max, fixture) => Math.max(max, fixture.number), 0) + 1
}

/** First free address in the universe that fits the footprint, or null. */
export function suggestAddress(universe: number, channels: number, ignoreId?: string) {
  const taken = patch.fixtures
    .filter((fixture) => fixture.universe === universe && fixture.id !== ignoreId)
    .map((fixture) => [fixture.startAddress, endAddress(fixture)] as const)
    .sort((a, b) => a[0] - b[0])
  let candidate = 1
  for (const [start, end] of taken) {
    if (candidate + channels - 1 < start) break
    candidate = Math.max(candidate, end + 1)
  }
  return candidate + channels - 1 <= DMX_SLOTS ? candidate : null
}

function saveLocal(fixture: FixtureInstance) {
  const index = patch.fixtures.findIndex((item) => item.id === fixture.id)
  if (index === -1) patch.fixtures.push({ ...fixture })
  else patch.fixtures[index] = { ...fixture }
}

/** Saves fixtures as one change. Resolves false (with `patch.error` set) if the backend rejects it. */
export async function saveFixtures(fixtures: FixtureInstance[]) {
  patch.error = ''
  if (!(inApp && patch.synced)) {
    fixtures.forEach(saveLocal)
    return true
  }
  const used = new Set(fixtures.map((f) => f.definitionId))
  const definitions = fixtureLibrary.filter((d) => used.has(d.id)).map(definitionToWire)
  try {
    applyWirePatch(
      await command<WirePatch>('save_fixtures', { definitions, fixtures: fixtures.map(toWire) }),
    )
    return true
  } catch (error) {
    patch.error = String(error)
    return false
  }
}

export async function removeFixtures(ids: string[]) {
  patch.error = ''
  if (!(inApp && patch.synced)) {
    patch.fixtures = patch.fixtures.filter((fixture) => !ids.includes(fixture.id))
    return true
  }
  try {
    applyWirePatch(await command<WirePatch>('remove_fixtures', { ids }))
    return true
  } catch (error) {
    patch.error = String(error)
    return false
  }
}

export function newFixtureId() {
  return crypto.randomUUID()
}
