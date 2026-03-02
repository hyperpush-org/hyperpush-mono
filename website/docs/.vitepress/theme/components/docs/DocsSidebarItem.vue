<script setup lang="ts">
import { computed } from 'vue'
import { useData } from 'vitepress'
import { isActive, type SidebarItem } from '@/composables/useSidebar'
import * as LucideIcons from 'lucide-vue-next'

const props = defineProps<{
  item: SidebarItem
}>()

const { page } = useData()

const active = computed(() => isActive(page.value.relativePath, props.item.link))

const iconComponent = computed(() => {
  if (!props.item.icon) return null
  return (LucideIcons as Record<string, unknown>)[props.item.icon] ?? null
})
</script>

<template>
  <div>
    <a
      :href="item.link"
      class="flex items-center gap-2 rounded-md px-2 py-1.5 text-[13px] transition-colors"
      :class="[
        active
          ? 'bg-accent text-foreground font-semibold'
          : 'text-muted-foreground hover:text-foreground hover:bg-accent',
      ]"
    >
      <component
        v-if="iconComponent"
        :is="iconComponent"
        class="size-3.5 shrink-0"
      />
      {{ item.text }}
    </a>
    <!-- Recursive children with left padding -->
    <ul v-if="item.items?.length" class="flex flex-col gap-0.5 pl-3 mt-0.5">
      <li v-for="child in item.items" :key="child.text">
        <DocsSidebarItem :item="child" />
      </li>
    </ul>
  </div>
</template>
