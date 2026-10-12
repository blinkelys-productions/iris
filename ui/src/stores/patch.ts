import { computed, reactive } from 'vue'

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

// Placeholder library until profiles are imported from the Rust GDTF importer.
export const fixtureLibrary: FixtureDefinition[] = [
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

export const patch = reactive({
  fixtures: [] as FixtureInstance[],
})

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

export function saveFixture(fixture: FixtureInstance) {
  const index = patch.fixtures.findIndex((item) => item.id === fixture.id)
  if (index === -1) patch.fixtures.push({ ...fixture })
  else patch.fixtures[index] = { ...fixture }
}

export function removeFixtures(ids: string[]) {
  patch.fixtures = patch.fixtures.filter((fixture) => !ids.includes(fixture.id))
}

export function newFixtureId() {
  return crypto.randomUUID()
}
