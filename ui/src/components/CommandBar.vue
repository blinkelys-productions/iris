<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { conflicts, patch } from '../stores/patch'
import { toggleMenu, workspace } from '../stores/workspace'

const command = ref('')
const time = ref('')
let timer: number | undefined

function tick() {
  time.value = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })
}

onMounted(() => {
  tick()
  timer = window.setInterval(tick, 1000)
})
onBeforeUnmount(() => window.clearInterval(timer))

const edge = 'grid w-10 shrink-0 place-items-center bg-zinc-800 text-zinc-300 hover:bg-zinc-700'
const stat = 'flex w-14 shrink-0 flex-col items-center justify-center bg-zinc-900 text-[10px] leading-tight tabular-nums text-zinc-400'
</script>

<template>
  <footer class="col-span-3 flex h-10 items-stretch gap-px border-t border-zinc-800 bg-zinc-950">
    <button :class="edge" aria-label="Collapse" disabled>◀</button>
    <span class="grid w-14 shrink-0 place-items-center bg-zinc-800 text-sm font-black tracking-tighter text-zinc-100" :title="workspace.showName">IRIS</span>
    <form class="flex min-w-0 flex-1 items-center gap-2 border border-zinc-600 bg-zinc-900 px-3" @submit.prevent="command = ''">
      <span class="font-mono text-sm text-zinc-100">{{ workspace.user }}<span class="text-accent">[Patch]</span>&gt;</span>
      <input
        v-model="command"
        class="min-w-0 flex-1 bg-transparent font-mono text-sm text-zinc-100 outline-none placeholder:text-zinc-600"
        aria-label="Command line"
        placeholder="Click here to create a new window"
      />
      <button type="button" class="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-300 hover:bg-zinc-700" @click="toggleMenu()">New window</button>
    </form>
    <span :class="stat" title="Clock"><span class="text-xs text-zinc-200">{{ time }}</span></span>
    <span :class="stat" title="Patched fixtures"><span class="text-xs text-zinc-200">{{ patch.fixtures.length }}</span>fx</span>
    <span :class="[stat, conflicts.length && '!text-red-400']" title="Patch conflicts"><span class="text-xs">{{ conflicts.length }}</span>⚠</span>
    <button :class="edge" aria-label="Expand" disabled>▶</button>
  </footer>
</template>
