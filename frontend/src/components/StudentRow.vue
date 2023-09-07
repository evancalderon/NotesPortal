<script setup lang="ts">
  import AddNote from './AddNote.vue'

  defineProps([
    'name',
    'time',
    'belt',
    'logins',
    'notes',
    'behaviours',
    'assigned',
    'assignable_to',
    'editor',
  ])
  var emit = defineEmits([
    'removeLogin',
    'removeNote',
    'removeBehaviour',
    'addLogin',
    'addNote',
    'addBehaviour',
    'editLogin',
    'editNote',
    'editBehaviour',
    'setAssigned',
  ])

  const colors: { [key: string]: string } = {
    black: 'black',
    red: 'firebrick',
    brown: 'sienna',
    purple: 'purple',
    blue: 'cornflowerblue',
    green: 'darkgreen',
    orange: '#E16E32',
    yellow: 'darkgoldenrod',
    white: 'darkgray',
  }
</script>

<template>
  <div>
    <p id="name">{{ name }}</p>
  </div>
  <p
    id="belt"
    :style="{ backgroundColor: colors[(belt as string).toLowerCase().split(' ')[0]] || colors['white'], color: 'white' }"
  >
    {{ belt }}
  </p>
  <div>
    <p id="time">{{ time }}</p>
  </div>
  <div>
    <AddNote
      id="logins"
      :values="logins"
      @remove="emit('removeLogin', $event)"
      @add="emit('addLogin', $event)"
      @edit="emit('editLogin', $event)"
    />
  </div>
  <div>
    <AddNote
      id="notes"
      :values="notes"
      show_extra="true"
      @remove="emit('removeNote', $event)"
      @add="emit('addNote', $event)"
      @edit="emit('editNote', $event)"
    />
  </div>
  <div>
    <AddNote
      id="behaviours"
      :values="behaviours"
      @remove="emit('removeBehaviour', $event)"
      @add="emit('addBehaviour', $event)"
      @edit="emit('editBehaviour', $event)"
    />
  </div>
  <div class="finalcol">
    <select
      name="assigned"
      id="assigned"
      :value="assigned"
      @change="emit('setAssigned', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="to in assignable_to" :key="to" :value="to">{{ to }}</option>
    </select>
  </div>
</template>

<style scoped lang="scss">
  p {
    white-space: nowrap;
  }

  .finalcol {
    width: 100px;

    > * {
      width: 100%;
    }
  }

  input:not([type='button']) {
    width: 100%;
  }

  #logins {
    width: 160px;
  }

  #behaviours {
    width: 200px;
  }
</style>
