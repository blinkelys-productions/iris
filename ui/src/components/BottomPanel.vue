<script setup lang="ts">
import { ref } from 'vue'

const groups = [
  { n: 1, name: 'Dimmer' },
  { n: 2, name: 'Position' },
  { n: 4, name: 'Color' },
  { n: 5, name: 'Beam' },
  { n: 6, name: 'Focus' },
  { n: 7, name: 'Control' },
]
const tools = ['Selection', 'Phaser', 'MAtricks', 'Prog Time', 'Exec Time']
const modes = [
  { name: 'Values and Timings', bar: 'before:bg-red-700' },
  { name: 'Phaser Overall', bar: 'before:bg-purple-600' },
  { name: 'Phaser Steps', bar: 'before:bg-purple-600' },
]
const valueModes = [
  { name: 'Absolute', bar: 'before:bg-orange-600' },
  { name: 'Relative', bar: 'before:bg-red-700' },
  { name: 'Fade', bar: 'before:bg-green-600' },
  { name: 'Delay', bar: 'before:bg-orange-500' },
]
const encoders = [
  { name: 'R', value: '100', unit: 'max', fill: 100 },
  { name: 'G', value: '0', unit: 'min', fill: 28 },
  { name: 'B', value: '0', unit: 'min', fill: 28 },
  { name: 'W', value: '0', unit: 'min', fill: 28 },
]

const activeGroup = ref(4)
const activeTool = ref('MAtricks')
const activeMode = ref('Values and Timings')
const activeValueMode = ref('Absolute')

const cell = 'relative flex items-center justify-center bg-zinc-800 text-center text-xs leading-[1.1] text-zinc-200 transition-colors hover:bg-zinc-700'
const bar = 'before:absolute before:inset-x-0 before:top-0 before:h-[3px]'
const on = '!text-accent'
const dot = 'absolute right-1.5 top-1.5 size-1.5 rounded-full border border-zinc-500'
</script>

<template>
  <section class="col-span-3 grid grid-cols-[84px_minmax(0,1fr)_96px] gap-px border-t border-zinc-800 bg-zinc-950" aria-label="Programmer controls (not yet connected)">
    <div class="grid grid-rows-[48px_48px_96px] gap-px">
      <button :class="[cell, on, 'rounded-none']">Sync</button>
      <button :class="[cell, 'rounded-none']">Single<br />Step</button>
      <div class="grid grid-rows-2 gap-px">
        <div :class="[cell, '!flex-col !items-start !justify-center px-2']">
          <span class="text-xs text-zinc-500">Align</span>
          <span>Off</span>
        </div>
        <div :class="[cell, '!flex-col !items-start !justify-center px-2']">
          <span class="text-xs text-zinc-500">Readout</span>
          <span>Natural</span>
        </div>
      </div>
    </div>

    <div class="grid min-w-0 grid-rows-[48px_48px_96px] gap-px">
      <div class="flex min-w-0 gap-px">
        <button
          v-for="group in groups"
          :key="group.n"
          :class="[cell, bar, 'flex-[3]', activeGroup === group.n ? [on, 'before:bg-red-700'] : 'before:bg-transparent']"
          @click="activeGroup = group.n"
        >
          <span class="absolute left-2 top-1.5 text-xs" :class="activeGroup === group.n ? 'text-accent' : 'text-zinc-400'">{{ group.n }}</span>
          <span :class="dot"></span>
          {{ group.name }}
        </button>
        <button
          v-for="tool in tools"
          :key="tool"
          :class="[cell, 'flex-[1.2] px-1', activeTool === tool && on]"
          @click="activeTool = activeTool === tool ? '' : tool"
        >
          {{ tool }}
        </button>
      </div>

      <div class="flex min-w-0 gap-px">
        <button :class="[cell, 'flex-[2.6] whitespace-nowrap']">RGB 1 of 2<span class="absolute right-1 top-1 text-[8px] text-zinc-500">▼</span></button>
        <button :class="[cell, 'flex-[3.4] !flex-col !justify-end pb-1.5']">
          <span class="absolute left-2 top-1.5 text-xs text-zinc-500">Link Values</span>
          Single
        </button>
        <button
          v-for="mode in modes"
          :key="mode.name"
          :class="[cell, bar, mode.bar, 'flex-[3] px-3', activeMode === mode.name && on]"
          @click="activeMode = mode.name"
        >
          {{ mode.name }}
        </button>
        <span class="grid w-4 shrink-0 place-items-center bg-zinc-950 text-[10px] text-zinc-500">▶</span>
        <button
          v-for="mode in valueModes"
          :key="mode.name"
          :class="[cell, bar, mode.bar, 'flex-[2.2] px-1', activeValueMode === mode.name && on]"
          @click="activeValueMode = mode.name"
        >
          {{ mode.name }}
        </button>
        <div class="flex flex-[2.6] gap-px">
          <button :class="[cell, 'flex-1 text-xl']">←</button>
          <span :class="[cell, 'flex-1 hover:!bg-zinc-800']">1</span>
          <button :class="[cell, 'flex-1 text-xl']">→</button>
        </div>
      </div>

      <div class="flex min-w-0 gap-px">
        <template v-for="encoder in encoders" :key="encoder.name">
          <div class="relative w-14 shrink-0 bg-zinc-800">
            <span class="absolute left-1.5 top-1.5 rounded-sm bg-zinc-900 px-2 text-xs text-accent">{{ encoder.name }}</span>
          </div>
          <div class="relative flex min-w-0 flex-[1_1_0%] flex-col overflow-hidden bg-zinc-800">
            <span class="relative z-10 mt-1.5 text-center text-xs text-zinc-200">{{ encoder.name }}</span>
            <span class="absolute right-1 top-1.5 h-1 w-3 bg-red-700"></span>
            <span class="absolute left-2 top-5 z-10 size-7 rounded-full border-2 border-zinc-400 bg-zinc-200"></span>
            <div class="absolute inset-x-0 bottom-0 bg-red-900" :style="{ height: `${encoder.fill}%` }"></div>
            <div class="relative mt-auto pb-2 text-center text-xs text-zinc-100">
              {{ encoder.value }} <span class="ml-1">{{ encoder.unit }}</span>
            </div>
          </div>
        </template>
        <div class="grid w-[22%] shrink-0 grid-rows-2 gap-px">
          <div v-for="axis in ['Y', 'X']" :key="axis" :class="[cell, 'justify-start gap-3 px-3 text-zinc-500']">
            <span class="size-7 shrink-0 rounded-full bg-zinc-600"></span>
            <span class="text-left">Screen {{ axis }}<br />Display 1</span>
          </div>
        </div>
      </div>
    </div>

    <div class="grid grid-rows-[96px_minmax(0,1fr)] gap-px">
      <div class="relative flex flex-col items-center justify-center bg-zinc-800 text-xs leading-[1.1] text-zinc-200">
        <span class="absolute right-1.5 top-1.5 size-1.5 bg-white"></span>
        <span>Grand</span>
        <span>Master</span>
      </div>
      <div class="flex items-start justify-center bg-zinc-900 pt-2 text-xs text-zinc-200">100%</div>
    </div>
  </section>
</template>
