<script setup lang="ts">
import { onMounted } from 'vue'
import {
  downloadShared,
  ownToken,
  retractShared,
  saveShareSettings,
  searchShare,
  share,
  uploadSharedFile,
} from '../stores/share'

defineEmits<{ close: [] }>()
onMounted(searchShare)

const kb = (bytes: number) => `${Math.max(1, Math.round(bytes / 1024))} KB`
</script>

<template>
  <div class="absolute inset-0 z-20 grid place-items-center bg-black/60 p-3">
    <div class="flex max-h-full w-[44rem] max-w-full flex-col gap-3 rounded-xl border border-zinc-700 bg-zinc-900 p-4 shadow-2xl">
      <div class="flex items-center">
        <h3 class="text-sm font-semibold text-zinc-100">GDTF Share</h3>
        <button class="btn !py-1 ml-auto text-xs" @click="$emit('close')">Close</button>
      </div>

      <div class="grid grid-cols-2 gap-2">
        <label class="grid gap-1 text-xs text-zinc-400">Server
          <input v-model="share.server" class="field" @change="saveShareSettings" @keydown.enter="searchShare" />
        </label>
        <label class="grid gap-1 text-xs text-zinc-400">Shared as
          <input v-model="share.uploader" class="field" placeholder="Your name (optional)" @change="saveShareSettings" />
        </label>
      </div>

      <form class="flex gap-1" @submit.prevent="searchShare">
        <input v-model="share.query" class="field flex-1" placeholder="Search manufacturer or fixture" />
        <select v-model="share.source" class="field" @change="searchShare">
          <option value="all">All</option>
          <option value="official">Official</option>
          <option value="community">Community</option>
        </select>
        <button class="btn btn-primary !py-1 text-xs" :disabled="share.busy">Search</button>
        <button type="button" class="btn !py-1 text-xs" :disabled="share.busy" @click="uploadSharedFile">Share a GDTF…</button>
      </form>

      <div v-if="share.error" class="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-red-300">{{ share.error }}</div>
      <div v-else-if="share.notice" class="rounded-md border border-zinc-700 bg-zinc-800/60 px-3 py-2 text-xs text-zinc-300">{{ share.notice }}</div>

      <div class="min-h-32 overflow-auto rounded-md border border-zinc-800">
        <table class="w-full text-left text-xs">
          <thead class="sticky top-0 bg-zinc-900 text-zinc-500">
            <tr><th class="p-2">Fixture</th><th class="p-2">Modes</th><th class="p-2">Source</th><th class="p-2"></th></tr>
          </thead>
          <tbody>
            <tr v-for="profile in share.results" :key="profile.id" class="border-t border-zinc-800">
              <td class="p-2 text-zinc-200">{{ profile.manufacturer }} <strong>{{ profile.name }}</strong>
                <div class="text-[10px] text-zinc-500">{{ profile.revision }} · {{ kb(profile.size) }} · {{ profile.downloads }} downloads</div>
              </td>
              <td class="p-2 text-zinc-400">{{ profile.modes.join(', ') }}</td>
              <td class="p-2">
                <span :class="profile.source === 'official' ? 'bg-violet-600/40 text-violet-200' : 'bg-zinc-700 text-zinc-300'" class="rounded-full px-2 py-0.5 text-[10px]">
                  {{ profile.source === 'official' ? 'Iris official' : profile.uploader }}
                </span>
              </td>
              <td class="whitespace-nowrap p-2 text-right">
                <button class="btn !py-1 text-xs" :disabled="share.busy" @click="downloadShared(profile)">Add to library</button>
                <button v-if="ownToken(profile.id)" class="btn !py-1 ml-1 text-xs hover:!border-red-500/60" :disabled="share.busy" @click="retractShared(profile)">Remove</button>
              </td>
            </tr>
            <tr v-if="!share.results.length && !share.busy">
              <td colspan="4" class="p-4 text-center text-zinc-500">No fixtures found.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
