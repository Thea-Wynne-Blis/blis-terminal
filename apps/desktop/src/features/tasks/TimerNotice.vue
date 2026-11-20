<script setup lang="ts">
defineProps<{ label: string }>()
</script>

<template>
  <main class="timer-notice" role="status">
    <strong>Blis 倒计时结束</strong>
    <p>{{ label }}</p>
  </main>
</template>

<style scoped>
.timer-notice {
  height: 100vh;
  overflow: hidden;
  padding: 12px 16px;
  border: 1px solid #56675f;
  border-inline-start: 3px solid var(--accent);
  background: var(--menu-surface);
}

.timer-notice strong {
  color: var(--accent);
}

.timer-notice p {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
  margin: 5px 0 0;
  overflow: hidden;
  overflow-wrap: anywhere;
}
</style>
