<script setup lang="ts">
import { onMounted, useTemplateRef } from 'vue'

defineProps<{ title: string }>()
const emit = defineEmits<{ confirm: []; cancel: [] }>()
const dialog = useTemplateRef<HTMLDialogElement>('note-removal-dialog')

onMounted(() => dialog.value?.showModal())
</script>

<template>
  <dialog
    ref="note-removal-dialog"
    class="note-editor note-removal-dialog"
    aria-labelledby="note-removal-title"
    aria-describedby="note-removal-description"
    @cancel.prevent="emit('cancel')"
  >
    <h2 id="note-removal-title">删除笔记</h2>
    <p id="note-removal-description">确定永久删除笔记「{{ title }}」吗？</p>
    <div class="note-actions">
      <button type="button" autofocus @click="emit('cancel')">取消</button>
      <button type="button" class="error" @click="emit('confirm')">删除</button>
    </div>
  </dialog>
</template>
