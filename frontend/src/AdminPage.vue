<script setup lang="ts">
  import { onMounted, ref } from 'vue'
  import Papa from 'papaparse'

  let invite_link = ref<string | null>(null)
  var loading = ref(false)
  var file_input = ref<HTMLInputElement>()
  var max_rows = ref(0)
  var file = ref()
  var rows = ref([] as unknown as string[][])
  var row_links = ref([1, 2, 3, 4, 5])
  var has_headers = ref(true)

  type User = {
    name: string
    role: string
  }
  let users = ref<User[]>([])
  onMounted(async () => {
    users.value = ((await (await fetch('/api/admin/users')).json()) as User[]).sort((a, b) =>
      b.name < a.name ? 1 : -1,
    )
  })

  async function createToken() {
    let req = await fetch('/api/admin/gen_token')
    let token = await req.json()
    return token
  }

  async function addUser() {
    window.location.assign('/create_user?token=' + (await createToken()))
  }

  async function genLink() {
    invite_link.value = `/create_user?token=` + (await createToken())
  }

  async function changePassword(name: String) {
    let new_pass = prompt('Enter a new password')
    if (new_pass !== null) {
      await fetch('/api/admin/change_pass', {
        body: `{"name":"${name}","pass":"${new_pass}"}`,
        headers: { ['Content-Type']: 'application/json' },
        method: 'POST',
      })
    }
  }

  async function deleteUser(name: String) {
    await fetch('/api/admin/delete_user', {
      body: `{"name":"${name}"}`,
      headers: { ['Content-Type']: 'application/json' },
      method: 'POST',
    })
    window.location.reload()
  }

  async function submitCsv() {
    let csv = await file_input.value?.files?.item(0)?.text()
    file.value = csv

    if (csv) {
      let data: string[][] = []
      let result = Papa.parse<string[]>(csv as string, {
        header: false,
      })
      if (has_headers.value) {
        result.data.splice(0, 1)
      }

      console.log(result.data)

      let links = row_links.value
      for (let v of result.data) {
        data.push([
          v[links[0] - 1],
          v[links[1] - 1],
          v[links[2] - 1],
          v[links[3] - 1],
          v[links[4] - 1],
        ])
        max_rows.value = v.length
      }

      rows.value = data
    }
  }

  async function uploadCsv() {
    let formData = new FormData()
    formData.append('row_data', JSON.stringify(row_links.value))
    formData.append('file', file.value as string)

    file.value = null
    max_rows.value = 0
    rows.value = []

    await fetch('/api/load_csv', {
      body: formData,
      method: 'POST',
      credentials: 'include',
    })
  }

  async function loadStudents() {
    loading.value = true
    await fetch('/api/admin/load_students', {
      method: 'POST',
      credentials: 'include',
    })
    loading.value = false
  }

  async function clearTimes() {
    await fetch('/api/admin/clear_times', {
      method: 'POST',
      credentials: 'include',
    })
  }
</script>

<template>
  <div class="container">
    <div class="root">
      <a href="/">Back</a>
      <h2>User Management</h2>
      <div class="left-align">
        <input type="button" value="Add User" @click="addUser()" />
        <input type="button" value="Generate Link" @click="genLink()" />
      </div>
      <p v-if="invite_link !== null">{{ invite_link }}</p>
      <table>
        <tr>
          <td>Username</td>
          <td colspan="2">Actions</td>
        </tr>
        <tr v-for="user in users" :key="user.name">
          <td>{{ user.name }}</td>
          <td>
            <input
              v-if="user.role != 'Admin'"
              type="button"
              value="Change Password"
              @click="changePassword(user.name)"
            />
            <input
              v-if="user.role != 'Admin'"
              type="button"
              value="Delete"
              @click="deleteUser(user.name)"
            />
          </td>
        </tr>
      </table>
      <h2>Students</h2>
      <div class="left-align">
        <input type="button" value="Submit CSV" @click="file_input?.click()" />
        <input
          type="file"
          name="file"
          id="file"
          ref="file_input"
          @change="submitCsv()"
          tabindex="-1"
          style="display: none"
        />
        <p v-if="loading">Loading...</p>
        <button v-else @click="loadStudents()">Load Students</button>
        <button @click="clearTimes()">Clear Times</button>
      </div>
      <h2>CSV Upload</h2>
      <div style="display: flex; flex-direction: row; gap: 6px">
        <input
          type="checkbox"
          id="has_headers"
          v-model="has_headers"
          :disabled="!file"
          @change="submitCsv()"
        />
        <label for="has_headers">Has headers</label>
      </div>
      <table>
        <tr>
          <td>Name</td>
          <td>Belt</td>
          <td>Logins</td>
          <td>Notes</td>
          <td>Behaviours</td>
        </tr>
        <tr>
          <td v-for="i in 5" :key="i">
            Col
            <input
              type="number"
              min="0"
              :max="max_rows"
              :disabled="max_rows == 0"
              v-model="row_links[i - 1]"
              @change="submitCsv()"
            />
          </td>
        </tr>
        <tr v-for="row in rows.filter((_, i) => i <= 4)" :key="row[0]">
          <td>{{ row[0] }}</td>
          <td>{{ row[1] }}</td>
          <td>{{ row[2] }}</td>
          <td>{{ row[3] }}</td>
          <td>{{ row[4] }}</td>
        </tr>
      </table>
      <div style="display: flex; flex-direction: row-reverse">
        <button @click="uploadCsv" :disabled="!file">Upload</button>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
  .root {
    display: flex;
    flex-direction: column;
    float: left;
    margin: 12px auto;
    padding: 8px;
    gap: 6px;
  }

  .container {
    display: flex;
    background-color: var(--color-background-soft);
    margin: 20px;
    padding: 0px;
  }

  .left-align {
    display: grid;
    grid-auto-flow: column;
    gap: 6px;
    align-content: left;
  }

  table,
  tr,
  td {
    border: 1px solid black;
  }

  td {
    gap: 2px;
  }

  table > button {
    border: 0px;
    margin: 0;
  }

  input[type='number'] {
    width: 50px;
  }
</style>
