<script setup lang="ts">
  import { ref } from 'vue'

  defineProps(['values', 'include_date', 'show_extra'])
  const emit = defineEmits(['remove', 'add', 'edit'])
  const value = ref('')
  const show_editor = ref(false)
  const editor_focused = ref(false)

  function addVal(val: string) {
    show_editor.value = false
    if (val == '') return

    emit('add', val)
    value.value = ''
  }

  function removeVal(val: string) {
    emit('remove', val)
  }

  function editVal(val: string) {
    emit('edit', val)
  }
</script>

<template>
  <div class="root">
    <div class="notes">
      <div class="initiate" @click="show_editor = !show_editor"></div>
      <template v-for="(val, index) in values" :key="index">
        <p class="note" @click="removeVal(val.id)" @contextmenu.prevent="editVal(val.id)">
          <span
            >{{ val.date && show_extra ? val.date + ': ' : '' }}{{ val.content
            }}{{ val.user && show_extra ? ' -- ' + val.user : '' }}</span
          >
        </p>
      </template>
    </div>
    <div class="break" :class="{ hide_editor: !show_editor && !editor_focused }"></div>
    <div class="inputs" :class="{ hide_editor: !show_editor && !editor_focused }">
      <input
        size="1"
        type="text"
        name="note"
        id="note"
        v-model="value"
        @focusin="editor_focused = true"
        @focusout="editor_focused = false"
        @keydown.enter="
          addVal(value)
          ;($event.target as HTMLElement).blur()
        "
      />
      <input size="1" type="button" value="+" id="add" @click="addVal(value)" />
    </div>
  </div>
</template>

<style scoped lang="scss">
  .root {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .row {
    display: block;
  }

  .notes {
    line-height: 1.5;
  }

  .initiate {
    z-index: 99;
    float: right;
    height: 1em;
    aspect-ratio: 1;
    background-color: var(--color-text);
    cursor: pointer;
  }

  .break {
    flex-basis: 100%;
    height: 0;
    border: 1px solid lightgrey;
  }

  .inputs {
    display: flex;
    gap: 6px;
    flex-direction: row;
  }

  .hide_editor {
    display: none;
  }

  p {
    flex-direction: column;
    line-height: 1.2;
    user-select: none;

    word-break: break-word;
  }

  p:hover {
    color: red;
    text-decoration: line-through;
    cursor: pointer;
  }

  #note {
    width: 100%;
    flex-grow: 1;
  }
</style>
