<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  closeShowDialog,
  deleteQuicksave,
  newShow,
  openShow,
  quicksave,
  renameShow,
  restoreBackup,
  restoreQuicksave,
  saveShow,
  saveShowAs,
  shows,
  type ShowDialogPage,
} from '../stores/shows'

const pages: { id: ShowDialogPage; title: string }[] = [
  { id: 'show', title: 'Show' },
  { id: 'quicksave', title: 'Quick Save' },
  { id: 'backup', title: 'Backup' },
]

const name = ref('')
const label = ref('')
const open = computed(() => shows.dialog !== null)
const hasShow = computed(() => shows.status !== null)

const fmt = (ms: number) => new Date(ms).toLocaleString()

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') closeShowDialog()
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    if (hasShow.value) saveShow()
  }
}

async function create() {
  if (!name.value.trim()) return
  await newShow(name.value.trim())
  name.value = ''
}

async function saveCopy() {
  if (!name.value.trim()) return
  await saveShowAs(name.value.trim())
  name.value = ''
}

async function rename() {
  if (!name.value.trim()) return
  await renameShow(name.value.trim())
  name.value = ''
}

async function makeQuicksave() {
  await quicksave(label.value.trim() || 'Quick Save')
  label.value = ''
}

onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div v-if="open" class="absolute inset-0 z-10 grid place-items-center bg-black/50 backdrop-blur-[2px]" @click.self="closeShowDialog">
    <section class="flex max-h-full w-[440px] max-w-full flex-col overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl" role="dialog" aria-label="Show files">
      <header class="flex items-center justify-between border-b border-zinc-800 py-2 pl-4 pr-2">
        <div>
          <h2 class="text-sm font-semibold text-zinc-100">
            {{ shows.status?.name ?? 'No show open' }}<span v-if="shows.status?.dirty" class="text-accent"> *</span>
          </h2>
          <small class="text-[11px] text-zinc-500">{{ shows.status?.dirty ? 'Unsaved changes' : 'All changes saved' }}</small>
        </div>
        <button class="grid size-7 place-items-center rounded-md text-xl leading-none text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100" aria-label="Close" @click="closeShowDialog">×</button>
      </header>

      <nav class="flex gap-1 border-b border-zinc-800 p-1.5">
        <button
          v-for="page in pages"
          :key="page.id"
          class="h-7 flex-1 rounded-md text-xs transition-colors"
          :class="shows.dialog === page.id ? 'bg-zinc-700 text-accent' : 'text-zinc-300 hover:bg-zinc-800'"
          @click="shows.dialog = page.id"
        >
          {{ page.title }}
        </button>
      </nav>

      <p v-if="shows.error" class="border-b border-zinc-800 bg-red-950/40 px-3 py-1.5 text-xs text-red-300">{{ shows.error }}</p>
      <p v-if="shows.status?.recovered_from" class="border-b border-zinc-800 bg-amber-950/40 px-3 py-1.5 text-xs text-amber-300">
        The saved file was damaged. Recovered from {{ shows.status.recovered_from }}.
      </p>

      <div class="grid min-h-0 gap-2 overflow-y-auto p-3 text-xs">
        <template v-if="shows.dialog === 'show'">
          <form class="flex gap-1.5" @submit.prevent="create">
            <input v-model="name" class="field flex-1" placeholder="Show name" aria-label="Show name" />
            <button class="btn btn-primary" type="submit" :disabled="!name.trim()">New</button>
            <button class="btn" type="button" :disabled="!hasShow || !name.trim()" @click="saveCopy">Save as</button>
            <button class="btn" type="button" :disabled="!hasShow || !name.trim()" @click="rename">Rename</button>
          </form>
          <button class="btn btn-primary" :disabled="!hasShow || !shows.status?.dirty" @click="saveShow">Save</button>
          <h3 class="mt-1 text-[11px] font-medium uppercase tracking-wide text-zinc-500">Saved shows</h3>
          <p v-if="!shows.list.length" class="text-zinc-500">No saved shows yet.</p>
          <button
            v-for="item in shows.list"
            :key="item.id"
            class="flex items-center justify-between rounded-lg border px-3 py-2 text-left transition-colors"
            :class="item.id === shows.status?.id ? 'border-accent bg-accent/10' : 'border-zinc-700 bg-zinc-800 hover:bg-zinc-700'"
            @click="openShow(item.id)"
          >
            <span class="text-zinc-100">{{ item.name }}</span>
            <span class="text-zinc-500">{{ fmt(item.saved_at_ms) }}</span>
          </button>
        </template>

        <template v-else-if="shows.dialog === 'quicksave'">
          <form class="flex gap-1.5" @submit.prevent="makeQuicksave">
            <input v-model="label" class="field flex-1" placeholder="Label (optional)" aria-label="Quicksave label" :disabled="!hasShow" />
            <button class="btn btn-primary" type="submit" :disabled="!hasShow">Quick Save</button>
          </form>
          <p v-if="!hasShow" class="text-zinc-500">Open or create a show first.</p>
          <p v-else-if="!shows.quicksaves.length" class="text-zinc-500">No quicksaves yet.</p>
          <div v-for="item in shows.quicksaves" :key="item.id" class="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2">
            <div class="min-w-0 flex-1">
              <div class="truncate text-zinc-100">{{ item.label }}</div>
              <div class="text-zinc-500">{{ item.kind === 'Auto' ? 'Automatic' : 'Manual' }} · {{ fmt(item.created_at_ms) }}</div>
            </div>
            <button class="btn" @click="restoreQuicksave(item.id)">Restore</button>
            <button class="btn" aria-label="Delete quicksave" @click="deleteQuicksave(item.id)">×</button>
          </div>
        </template>

        <template v-else>
          <p v-if="!hasShow" class="text-zinc-500">Open or create a show first.</p>
          <p v-else-if="!shows.backups.length" class="text-zinc-500">Backups appear here each time the show is saved over an earlier version.</p>
          <div v-for="item in shows.backups" :key="item.id" class="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2">
            <span class="flex-1 text-zinc-100">{{ fmt(item.saved_at_ms) }}</span>
            <button class="btn" @click="restoreBackup(item.id)">Restore</button>
          </div>
        </template>
      </div>
    </section>
  </div>
</template>
