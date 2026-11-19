<script setup lang="ts">
import { onMounted, useTemplateRef } from 'vue'

defineProps<{ title: string; description: string; confirmLabel: string }>()
const emit = defineEmits<{ confirm: []; cancel: [] }>()
const dialog = useTemplateRef<HTMLDialogElement>('removal-dialog')

onMounted(() => dialog.value?.showModal())
</script>

<template>
  <dialog
    ref="removal-dialog"
    class="note-editor removal-dialog"
    aria-labelledby="removal-title"
    aria-describedby="removal-description"
    @cancel.prevent="emit('cancel')"
  >
    <h2 id="removal-title">{{ title }}</h2>
    <p id="removal-description">{{ description }}</p>
    <div class="note-actions">
      <button type="button" autofocus @click="emit('cancel')">取消</button>
      <button type="button" class="error" @click="emit('confirm')">
        {{ confirmLabel }}
      </button>
    </div>
  </dialog>
</template>
