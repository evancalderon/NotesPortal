<script setup lang="ts">
  import StudentGrid from './components/StudentGrid.vue'
</script>

<template>
  <div class="root">
    <Suspense><StudentGrid /></Suspense>
  </div>
</template>

<style scoped lang="scss">
  .root {
    background-color: var(--color-background-soft);
    margin: 20px;
    padding: 8px;
    gap: 6px;
  }
</style>
