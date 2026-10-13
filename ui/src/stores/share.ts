import { open } from '@tauri-apps/plugin-dialog'
import { reactive } from 'vue'
import { command } from './backend'
import { addToLibrary, type WireDefinition } from './patch'

export interface SharedProfile {
  id: string
  manufacturer: string
  name: string
  revision: string
  modes: string[]
  source: 'official' | 'community'
  uploader: string
  downloads: number
  size: number
}

interface ShareUpload {
  profile: SharedProfile
  delete_token: string | null
  duplicate: boolean
}

const DEFAULT_SERVER = 'http://127.0.0.1:8787'

const load = <T>(key: string, fallback: T): T => {
  try {
    return JSON.parse(localStorage.getItem(key) ?? '') as T
  } catch {
    return fallback
  }
}

export const share = reactive({
  server: localStorage.getItem('iris.share.server') ?? DEFAULT_SERVER,
  uploader: localStorage.getItem('iris.share.uploader') ?? '',
  query: '',
  source: 'all' as 'all' | 'official' | 'community',
  results: [] as SharedProfile[],
  /** Delete tokens for files this console uploaded, keyed by server then profile id. */
  tokens: load<Record<string, Record<string, string>>>('iris.share.tokens', {}),
  busy: false,
  error: '',
  notice: '',
})

export const saveShareSettings = () => {
  localStorage.setItem('iris.share.server', share.server.trim())
  localStorage.setItem('iris.share.uploader', share.uploader.trim())
}

export const ownToken = (id: string) => share.tokens[share.server.trim()]?.[id]

async function run(action: () => Promise<string | void>) {
  share.busy = true
  share.error = ''
  share.notice = ''
  try {
    const notice = await action()
    if (notice) share.notice = notice
  } catch (error) {
    share.error = String(error)
  } finally {
    share.busy = false
  }
}

const args = () => ({ server: share.server.trim() })

export const searchShare = () =>
  run(async () => {
    saveShareSettings()
    share.results = await command<SharedProfile[]>('share_search', {
      ...args(),
      query: share.query,
      source: share.source === 'all' ? '' : share.source,
    })
  })

export const downloadShared = (profile: SharedProfile) =>
  run(async () => {
    addToLibrary(await command<WireDefinition>('share_download', { ...args(), id: profile.id }))
    return `Added ${profile.manufacturer} ${profile.name} to your fixture library.`
  })

function remember(result: ShareUpload) {
  if (result.delete_token) {
    const server = share.server.trim()
    share.tokens[server] = { ...share.tokens[server], [result.profile.id]: result.delete_token }
    localStorage.setItem('iris.share.tokens', JSON.stringify(share.tokens))
  }
  return result.duplicate
    ? `${result.profile.name} is already shared.`
    : `Shared ${result.profile.manufacturer} ${result.profile.name}.`
}

export const uploadSharedFile = () =>
  run(async () => {
    const picked = await open({ multiple: true, filters: [{ name: 'GDTF fixture profile', extensions: ['gdtf'] }] })
    const paths = picked ? (Array.isArray(picked) ? picked : [picked]) : []
    const notices: string[] = []
    for (const path of paths) {
      saveShareSettings()
      notices.push(remember(await command<ShareUpload>('share_upload_file', { ...args(), path, uploader: share.uploader })))
    }
    if (paths.length) share.results = await command<SharedProfile[]>('share_search', { ...args(), query: share.query, source: share.source === 'all' ? '' : share.source })
    return notices.join(' ')
  })

export const retractShared = (profile: SharedProfile) =>
  run(async () => {
    const token = ownToken(profile.id)
    if (!token) throw new Error('this console did not upload that file')
    await command('share_delete', { ...args(), id: profile.id, token })
    delete share.tokens[share.server.trim()]![profile.id]
    localStorage.setItem('iris.share.tokens', JSON.stringify(share.tokens))
    share.results = share.results.filter((r) => r.id !== profile.id)
    return 'Removed your shared file.'
  })
