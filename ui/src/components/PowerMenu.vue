<script setup lang="ts">
import { restart, shutOff, workspace } from '../stores/workspace'
</script>

<template>
  <div v-if="workspace.powerOpen" class="fixed inset-0 z-20" @click.self="workspace.powerOpen = false">
    <section class="absolute left-11 top-1 flex w-36 flex-col gap-1 rounded-xl border border-zinc-700 bg-zinc-900 p-1.5 shadow-2xl" role="dialog" aria-label="Power">
      <button class="btn !justify-start" @click="shutOff">Shut off</button>
      <button class="btn !justify-start" @click="restart">Restart</button>
    </section>
  </div>
  <div v-if="workspace.poweredOff" class="fixed inset-0 z-30 grid place-items-center bg-black">
    <button class="btn" @click="workspace.poweredOff = false">Power on</button>
  </div>
</template>
