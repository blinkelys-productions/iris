import { reactive, watch } from 'vue'
import { command, inApp } from './backend'
import { loadPatch, loadProfiles } from './patch'
import { applyLayout, snapshotLayout, workspace, type WorkspaceLayout } from './workspace'

export interface SessionStatus {
  id: string
  name: string
  dirty: boolean
  recovered_from: string | null
}

export interface ShowSummary {
  id: string
  name: string
  app_version: string
  saved_at_ms: number
}

export interface QuicksaveSummary {
  id: string
  kind: 'Manual' | 'Auto'
  label: string
  created_at_ms: number
}

export interface BackupSummary {
  id: string
  saved_at_ms: number
}

export type ShowDialogPage = 'show' | 'quicksave' | 'backup'

export { inApp }

export const shows = reactive({
  dialog: null as ShowDialogPage | null,
  status: null as SessionStatus | null,
  list: [] as ShowSummary[],
  quicksaves: [] as QuicksaveSummary[],
  backups: [] as BackupSummary[],
  error: '',
})

async function call<T>(name: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!inApp) {
    shows.error = 'Show storage is only available in the Iris app.'
    return undefined
  }
  try {
    shows.error = ''
    return await command<T>(name, args)
  } catch (error) {
    shows.error = String(error)
    return undefined
  }
}

let lastSyncedLayout = ''

/** Loads the open show's patch and workspace layout into the interface. */
async function loadSession() {
  try {
    await loadProfiles()
    await loadPatch()
    const layout = await command<WorkspaceLayout | null>('get_workspace')
    applyLayout(layout)
    lastSyncedLayout = JSON.stringify(snapshotLayout())
  } catch (error) {
    shows.error = String(error)
  }
}

async function adopt(status: SessionStatus | undefined) {
  if (!status) return false
  shows.status = status
  workspace.showName = status.name
  await loadSession()
  return true
}

export async function refresh() {
  const status = await call<SessionStatus | null>('session_status')
  if (status !== undefined) {
    shows.status = status
    if (status) workspace.showName = status.name
  }
  const list = await call<ShowSummary[]>('list_shows')
  if (list) shows.list = list
  if (shows.status) {
    const quicksaves = await call<QuicksaveSummary[]>('list_quicksaves')
    if (quicksaves) shows.quicksaves = quicksaves
    const backups = await call<BackupSummary[]>('list_backups')
    if (backups) shows.backups = backups
  } else {
    shows.quicksaves = []
    shows.backups = []
  }
}

export async function openShowDialog(page: ShowDialogPage) {
  workspace.menuOpen = false
  shows.dialog = page
  await refresh()
}

export function closeShowDialog() {
  shows.dialog = null
}

async function run(name: string, args?: Record<string, unknown>) {
  if (await adopt(await call<SessionStatus>(name, args))) await refresh()
}

/** On startup, reopens the most recent show, or creates one so edits are always stored. */
export async function bootstrap() {
  if (!inApp) return
  const status = await call<SessionStatus | null>('session_status')
  if (status) {
    await adopt(status)
  } else {
    const list = await call<ShowSummary[]>('list_shows')
    if (list?.length) await adopt(await call<SessionStatus>('open_show', { id: list[0]!.id }))
    else await adopt(await call<SessionStatus>('new_show', { name: 'Untitled Show' }))
  }
  await refresh()
  startLayoutSync()
}

let syncTimer: ReturnType<typeof setTimeout> | undefined

function startLayoutSync() {
  watch(
    () => JSON.stringify(snapshotLayout()),
    (json) => {
      if (!shows.status || json === lastSyncedLayout) return
      clearTimeout(syncTimer)
      syncTimer = setTimeout(async () => {
        const status = await call<SessionStatus>('set_workspace', { workspace: JSON.parse(json) })
        if (status) {
          lastSyncedLayout = json
          shows.status = status
        }
      }, 400)
    },
  )
}

export const newShow = (name: string) => run('new_show', { name })
export const openShow = (id: string) => run('open_show', { id })
export const saveShow = () => run('save_show')
export const saveShowAs = (name: string) => run('save_show_as', { name })
export const renameShow = (name: string) => run('rename_show', { name })
export const restoreQuicksave = (id: string) => run('restore_quicksave', { id })
export const restoreBackup = (id: string) => run('restore_backup', { id })

export async function quicksave(label: string) {
  const list = await call<QuicksaveSummary[]>('quicksave', { label })
  if (list) {
    shows.quicksaves = list
    await refresh()
  }
}

export async function deleteQuicksave(id: string) {
  const list = await call<QuicksaveSummary[]>('delete_quicksave', { id })
  if (list) shows.quicksaves = list
}
