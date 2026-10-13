import { invoke } from '@tauri-apps/api/core'

export const inApp = '__TAURI_INTERNALS__' in window

/** Calls a backend command; rejects with the backend's message. */
export function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(name, args)
}
