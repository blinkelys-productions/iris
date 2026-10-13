import { channelCount, findDefinition, type FixtureInstance } from './patch'

export type BulkField = 'number' | 'name' | 'type' | 'mode' | 'universe' | 'address'

export interface BulkEdit {
  field: BulkField
  /** Text or number entry for number, name, universe and address. */
  text: string
  /** Target fixture type, for `type`. */
  definitionId: string
  /** Target mode name, for `mode` and `type`. */
  modeName: string
  /** Number, name or address each fixture in turn instead of giving them all the same value. */
  sequence: boolean
}

export const fieldLabels: Record<BulkField, string> = {
  number: 'Fixture ID',
  name: 'Name',
  type: 'Fixture type',
  mode: 'DMX mode',
  universe: 'Universe',
  address: 'Address',
}

/** Rows in `order` between two ids, inclusive, in either direction. */
export function rangeBetween(order: string[], a: string, b: string) {
  const from = order.indexOf(a)
  const to = order.indexOf(b)
  if (from === -1 || to === -1) return [b]
  return order.slice(Math.min(from, to), Math.max(from, to) + 1)
}

function withMode(fixture: FixtureInstance, definitionId: string, modeName: string): FixtureInstance {
  const modes = findDefinition(definitionId)?.modes ?? []
  const mode = modes.find((m) => m.name === modeName) ?? (definitionId !== fixture.definitionId ? modes[0] : undefined)
  return mode ? { ...fixture, definitionId, modeId: mode.id } : fixture
}

/** Counts up a trailing number ("Quantum 1" -> "Quantum 2"); names without one are left as typed. */
export function numberedName(name: string, offset: number) {
  const match = /^(.*?)(\d+)$/.exec(name.trim())
  if (!match) return name.trim()
  const digits = match[2]!
  return `${match[1]}${String(Number(digits) + offset).padStart(digits.length, '0')}`
}

/** Applies one edit to fixtures that are already in display order; returns new objects. */
export function applyBulkEdit(fixtures: FixtureInstance[], edit: BulkEdit): FixtureInstance[] {
  const value = Number(edit.text)
  const sequential = edit.sequence && fixtures.length > 1
  let nextAddress = value
  return fixtures.map((fixture, index) => {
    switch (edit.field) {
      case 'number':
        return { ...fixture, number: value + (sequential ? index : 0) }
      case 'name': {
        const name = edit.text.trim()
        return { ...fixture, name: numberedName(name, index) }
      }
      case 'universe':
        return { ...fixture, universe: value }
      case 'address': {
        const startAddress = sequential ? nextAddress : value
        nextAddress = startAddress + channelCount(fixture)
        return { ...fixture, startAddress }
      }
      case 'type':
        return withMode(fixture, edit.definitionId, edit.modeName)
      case 'mode':
        return withMode(fixture, fixture.definitionId, edit.modeName)
    }
  })
}
