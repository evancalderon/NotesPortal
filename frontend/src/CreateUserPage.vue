<script setup lang="ts">
  import { onMounted } from 'vue'

  onMounted(function () {
    let params = new URLSearchParams(window.location.search)

    let token = document.getElementById('token') as any
    let token_value = params.get('token') as any
    token.value = token_value
  })
</script>

<template>
  <div class="root">
    <form method="post" action="/api/invite">
      <input type="hidden" name="token" id="token" value="" />
      <label for="name">Username</label>
      <input type="text" name="name" />
      <label for="pass">Password</label>
      <input type="password" name="pass" />
      <div style="display: block">
        <input type="submit" value="Create User" style="float: right" />
      </div>
    </form>
  </div>
</template>

<style scoped lang="scss">
  .root {
    display: flex;
    background-color: var(--color-background-soft);
    margin: 20px;
    padding: 0px;
  }

  form {
    margin: 20px auto;
    display: grid;
    gap: 6px;
    width: 400px;
  }
</style>
