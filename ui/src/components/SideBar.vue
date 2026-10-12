<script setup lang="ts">
import Icon from './Icon.vue'
import { menuPages, toggleMenu, togglePower, workspace } from '../stores/workspace'

const base = 'grid size-8 place-items-center rounded-md text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100'
</script>

<template>
  <aside class="flex flex-col items-center gap-0.5 border-r border-zinc-800 bg-zinc-900 p-1" aria-label="Navigation">
    <button
      :class="[base, workspace.powerOpen && 'bg-zinc-800 !text-accent']"
      aria-label="Power"
      title="Power"
      :aria-expanded="workspace.powerOpen"
      @click="togglePower"
    >
      <Icon name="power" />
    </button>

    <span class="my-0.5 h-px w-5 bg-zinc-800"></span>

    <button
      v-for="page in menuPages"
      :key="page.id"
      :class="[base, workspace.menuOpen && workspace.menuPage === page.id && 'bg-zinc-800 !text-accent']"
      :aria-label="page.title"
      :title="page.title"
      :aria-expanded="workspace.menuOpen && workspace.menuPage === page.id"
      @click="toggleMenu(page.id)"
    >
      <Icon :name="page.icon" />
    </button>
  </aside>
</template>
