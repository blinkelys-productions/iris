import { open, save } from '@tauri-apps/plugin-dialog'
import { command } from './backend'
import { importGdtfFile, importMvrFile, patch } from './patch'
import { shows } from './shows'

/** Runs an interchange action, reporting its outcome through the patch notice and error. */
async function run(action: () => Promise<string | null>) {
  patch.error = ''
  patch.notice = ''
  try {
    const notice = await action()
    if (notice) patch.notice = notice
  } catch (error) {
    patch.error = String(error)
  }
}

const baseName = () => shows.status?.name || 'patch'

export const importGdtf = () =>
  run(async () => {
    const picked = await open({
      multiple: true,
      filters: [{ name: 'GDTF fixture profile', extensions: ['gdtf'] }],
    })
    const paths = picked ? (Array.isArray(picked) ? picked : [picked]) : []
    for (const path of paths) await importGdtfFile(path)
    return paths.length ? `Imported ${paths.length} fixture profile${paths.length === 1 ? '' : 's'}.` : null
  })

export const importMvr = () =>
  run(async () => {
    const path = await open({ filters: [{ name: 'MVR patch', extensions: ['mvr'] }] })
    if (typeof path !== 'string') return null
    const result = await importMvrFile(path)
    return [`Imported ${result.fixtures_added} fixtures.`, ...result.warnings].join(' ')
  })

export const exportMvr = () =>
  run(async () => {
    const path = await save({
      defaultPath: `${baseName()}.mvr`,
      filters: [{ name: 'MVR patch', extensions: ['mvr'] }],
    })
    if (!path) return null
    await command('export_mvr', { path })
    return `Exported MVR to ${path}`
  })

export const exportPatchPdf = () =>
  run(async () => {
    const path = await save({
      defaultPath: `${baseName()} patch.pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    })
    if (!path) return null
    await command('export_patch_pdf', { path })
    return `Exported patch list to ${path}`
  })
