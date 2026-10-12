import { invoke } from '@tauri-apps/api/core'
import { reactive } from 'vue'
import { workspace } from './workspace'

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

export const inApp = '__TAURI_INTERNALS__' in window

export const shows = reactive({
  dialog: null as ShowDialogPage | null,
  status: null as SessionStatus | null,
  list: [] as ShowSummary[],
  quicksaves: [] as QuicksaveSummary[],
  backups: [] as BackupSummary[],
  error: '',
})

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!inApp) {
    shows.error = 'Show storage is only available in the Iris app.'
    return undefined
  }
  try {
    shows.error = ''
    return await invoke<T>(command, args)
  } catch (error) {
    shows.error = String(error)
    return undefined
  }
}

function adopt(status: SessionStatus | undefined) {
  if (!status) return false
  shows.status = status
  workspace.showName = status.name
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

async function run(command: string, args?: Record<string, unknown>) {
  if (adopt(await call<SessionStatus>(command, args))) await refresh()
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
