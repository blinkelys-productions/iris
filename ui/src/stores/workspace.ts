import { computed, reactive } from 'vue'

export type WindowKind =
  | 'patch'
  | 'livePatch'
  | 'fixtureTypes'
  | 'programmer'
  | 'playbacks'
  | 'output'
  | 'groups'
  | 'fixtures'
  | 'presetDimmer'
  | 'presetPosition'
  | 'presetColor'
  | 'presetBeam'
  | 'presetFocus'
  | 'presetControl'
  | 'dmxChart'

export type IconName =
  | 'power'
  | 'menu'
  | 'settings'
  | 'patch'
  | 'fixtureTypes'
  | 'programmer'
  | 'playbacks'
  | 'output'
  | 'help'
  | 'masters'
  | 'screens'
  | 'console'

export interface MenuEntry {
  kind: WindowKind
  title: string
  description: string
  icon: IconName
  available: boolean
}

export const menuEntries: MenuEntry[] = [
  { kind: 'patch', title: 'Patch', description: 'Fixtures, universes and addresses', icon: 'patch', available: true },
  { kind: 'livePatch', title: 'Live Patch', description: 'Patch while outputting', icon: 'patch', available: false },
  { kind: 'fixtureTypes', title: 'Fixture Types', description: 'Profiles and modes', icon: 'fixtureTypes', available: false },
  { kind: 'programmer', title: 'Programmer', description: 'Select and edit attributes', icon: 'programmer', available: false },
  { kind: 'playbacks', title: 'Playbacks', description: 'Cues and sequences', icon: 'playbacks', available: false },
  { kind: 'output', title: 'DMX Output', description: 'sACN and Art-Net', icon: 'output', available: false },
  { kind: 'groups', title: 'Groups', description: 'Fixture selections', icon: 'fixtureTypes', available: true },
  { kind: 'fixtures', title: 'Fixtures', description: 'Patched fixtures', icon: 'fixtureTypes', available: true },
  { kind: 'presetDimmer', title: 'Dimmer Presets', description: 'Dimmer values', icon: 'programmer', available: true },
  { kind: 'presetPosition', title: 'Position Presets', description: 'Pan and tilt values', icon: 'programmer', available: true },
  { kind: 'presetColor', title: 'Color Presets', description: 'Color values', icon: 'programmer', available: true },
  { kind: 'presetBeam', title: 'Beam Presets', description: 'Beam values', icon: 'programmer', available: true },
  { kind: 'presetFocus', title: 'Focus Presets', description: 'Focus values', icon: 'programmer', available: true },
  { kind: 'presetControl', title: 'Control Presets', description: 'Control values', icon: 'programmer', available: true },
  { kind: 'dmxChart', title: 'DMX Chart', description: 'Universe address map', icon: 'output', available: true },
]

export interface ModuleGroup {
  title: string
  kinds: WindowKind[]
}

export const moduleGroups: ModuleGroup[] = [
  { title: 'Pools', kinds: ['groups', 'fixtures'] },
  { title: 'Presets', kinds: ['presetDimmer', 'presetPosition', 'presetColor', 'presetBeam', 'presetFocus', 'presetControl'] },
  { title: 'Windows', kinds: ['dmxChart', 'patch', 'livePatch'] },
]

export const GRID_COLS = 12
export const GRID_ROWS = 8

const defaultSize: Partial<Record<WindowKind, [number, number]>> = {
  patch: [12, 8],
  livePatch: [12, 5],
  fixtures: [4, 4],
  dmxChart: [6, 4],
}

export type MenuPage = 'menu' | 'masters' | 'screens' | 'console' | 'help'

export const menuPages: { id: MenuPage; title: string; icon: IconName }[] = [
  { id: 'menu', title: 'Menu', icon: 'menu' },
  { id: 'masters', title: 'Masters', icon: 'masters' },
  { id: 'screens', title: 'Screens', icon: 'screens' },
  { id: 'console', title: 'Console', icon: 'console' },
  { id: 'help', title: 'Help', icon: 'help' },
]

export interface PlacedWindow {
  id: number
  kind: WindowKind
  col: number
  row: number
  w: number
  h: number
}

interface View {
  windows: PlacedWindow[]
  focused: number | null
}

const INITIAL_VIEWS = 12
const newView = (): View => ({ windows: [], focused: null })
let nextWindowId = 1

export const workspace = reactive({
  menuOpen: false,
  menuPage: 'menu' as MenuPage,
  powerOpen: false,
  poweredOff: false,
  activeView: 0,
  picker: null as { col: number; row: number } | null,
  views: Array.from({ length: INITIAL_VIEWS }, newView),
  showName: 'Untitled Show',
  user: 'Admin',
})

export const currentView = computed(() => workspace.views[workspace.activeView]!)

// Plain Patch edits offline, so the programmer panel is hidden; Live Patch keeps it.
export const showBottomPanel = computed(() => {
  const kinds = currentView.value.windows.map((window) => window.kind)
  return !kinds.includes('patch') || kinds.includes('livePatch')
})

export function titleFor(kind: WindowKind) {
  return menuEntries.find((entry) => entry.kind === kind)?.title ?? kind
}

function isFree(view: View, col: number, row: number, w: number, h: number) {
  if (col + w > GRID_COLS || row + h > GRID_ROWS) return false
  return view.windows.every(
    (win) => col + w <= win.col || win.col + win.w <= col || row + h <= win.row || win.row + win.h <= row,
  )
}

export function isCellFree(col: number, row: number) {
  return isFree(currentView.value, col, row, 1, 1)
}

// Tries the default size first, then smaller footprints, largest area first.
function place(kind: WindowKind, at?: { col: number; row: number }) {
  const view = currentView.value
  const [dw, dh] = defaultSize[kind] ?? [4, 3]
  const sizes: [number, number][] = []
  for (let w = dw; w >= 1; w--) for (let h = dh; h >= 1; h--) sizes.push([w, h])
  sizes.sort((a, b) => b[0] * b[1] - a[0] * a[1])

  for (const [w, h] of sizes) {
    if (at) {
      if (isFree(view, at.col, at.row, w, h)) return { col: at.col, row: at.row, w, h }
      continue
    }
    if (w < 3 || h < 2) continue
    for (let row = 0; row < GRID_ROWS; row++)
      for (let col = 0; col < GRID_COLS; col++) if (isFree(view, col, row, w, h)) return { col, row, w, h }
  }
  return at ? { col: at.col, row: at.row, w: 1, h: 1 } : { col: 0, row: 0, w: dw, h: dh }
}

export function addWindow(kind: WindowKind, at?: { col: number; row: number }) {
  const view = currentView.value
  const win: PlacedWindow = { id: nextWindowId++, kind, ...place(kind, at) }
  view.windows.push(win)
  view.focused = win.id
  workspace.menuOpen = false
  workspace.picker = null
}

// Menu entries focus an existing window of the same kind instead of duplicating it.
export function openWindow(kind: WindowKind) {
  const existing = currentView.value.windows.find((window) => window.kind === kind)
  if (existing) {
    currentView.value.focused = existing.id
    workspace.menuOpen = false
  } else addWindow(kind)
}

export function closeWindow(id: number) {
  const view = currentView.value
  view.windows = view.windows.filter((item) => item.id !== id)
  if (view.focused === id) view.focused = view.windows.at(-1)?.id ?? null
}

export function focusWindow(id: number) {
  currentView.value.focused = id
}

export function openPicker(col: number, row: number) {
  workspace.menuOpen = false
  workspace.powerOpen = false
  workspace.picker = { col, row }
}

export function closePicker() {
  workspace.picker = null
}

export function addView() {
  workspace.views.push(newView())
  workspace.activeView = workspace.views.length - 1
  workspace.picker = null
}

export function selectView(index: number) {
  workspace.activeView = index
  workspace.menuOpen = false
  workspace.picker = null
}

export function toggleMenu(page: MenuPage = 'menu') {
  workspace.menuOpen = !(workspace.menuOpen && workspace.menuPage === page)
  workspace.menuPage = page
  workspace.powerOpen = false
  workspace.picker = null
}

export function togglePower() {
  workspace.powerOpen = !workspace.powerOpen
  workspace.picker = null
  workspace.menuOpen = false
}

export function shutOff() {
  workspace.powerOpen = false
  workspace.poweredOff = true
}

export function restart() {
  workspace.powerOpen = false
  window.location.reload()
}
