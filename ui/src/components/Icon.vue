<script setup lang="ts">
import type { IconName } from '../stores/workspace'

defineProps<{ name: IconName }>()
</script>

<template>
  <svg class="size-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <template v-if="name === 'power'">
      <path d="M12 3v8" />
      <path d="M6.5 6.8a8 8 0 1 0 11 0" />
    </template>
    <template v-else-if="name === 'menu'">
      <path d="M4 7h16M4 12h16M4 17h16" />
    </template>
    <template v-else-if="name === 'settings'">
      <circle cx="12" cy="12" r="3.2" />
      <path d="M12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M18.4 5.6l-1.8 1.8M7.4 16.6l-1.8 1.8" />
    </template>
    <template v-else-if="name === 'patch'">
      <rect x="5" y="4" width="14" height="16" rx="1.5" />
      <path d="M9 4v16M12 9h4M12 13h4" />
    </template>
    <template v-else-if="name === 'fixtureTypes'">
      <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z" />
      <path d="M4 7.5l8 4.5 8-4.5M12 12v9" />
    </template>
    <template v-else-if="name === 'programmer'">
      <path d="M6 4v16M12 4v16M18 4v16" />
      <path d="M4 9h4M10 15h4M16 8h4" />
    </template>
    <template v-else-if="name === 'playbacks'">
      <rect x="4" y="4" width="6" height="6" />
      <rect x="14" y="4" width="6" height="6" />
      <rect x="4" y="14" width="6" height="6" />
      <rect x="14" y="14" width="6" height="6" />
    </template>
    <template v-else-if="name === 'output'">
      <rect x="3" y="5" width="18" height="12" rx="1.5" />
      <path d="M9 21h6M12 17v4" />
    </template>
    <template v-else-if="name === 'masters'">
      <path d="M8 4v16M16 4v16" />
      <rect x="5.5" y="14" width="5" height="3" />
      <rect x="13.5" y="7" width="5" height="3" />
    </template>
    <template v-else-if="name === 'screens'">
      <rect x="3" y="4" width="8" height="6" rx="1" />
      <rect x="13" y="4" width="8" height="6" rx="1" />
      <rect x="3" y="14" width="8" height="6" rx="1" />
      <rect x="13" y="14" width="8" height="6" rx="1" />
    </template>
    <template v-else-if="name === 'console'">
      <rect x="3" y="6" width="18" height="12" rx="1.5" />
      <path d="M7 10v4M11 10v4M15 12h2" />
    </template>
    <template v-else-if="name === 'help'">
      <path d="M9 9a3 3 0 1 1 4.3 2.7c-.8.5-1.3 1.1-1.3 2.3" />
      <path d="M12 18h.01" />
    </template>
  </svg>
</template>
