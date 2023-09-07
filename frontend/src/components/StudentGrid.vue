<script setup lang="ts">
  import { inject, ref } from 'vue'
  import type { VueCookies } from 'vue-cookies'
  import StudentRow from './StudentRow.vue'
  ;(clearTimeout as () => void)()

  var logging_in = ref(-1)
  var filter_by = ref(localStorage.getItem('filter_by') || 'today')

  const $cookies = inject<VueCookies>('$cookies')
  if ($cookies?.get('token') !== '') {
    logging_in.value = 2
  }

  type Note = {
    id: string
    date: string
    user: string
    content: string
  }

  type Student = {
    id: string
    first_name: string
    last_name: string
    name: string
    time: string
    date: string | null
    belt: string
    logins: Note[]
    notes: Note[]
    behaviours: Note[]
    assigned: string
  }

  var timer: ReturnType<typeof setTimeout>
  var students = ref<Student[]>([])
  var senseis = ref<String[]>([])
  var secret = ref('')
  var username = ref('')
  var role = ref('')

  try {
    senseis.value = await (
      await fetch('/api/senseis', {
        credentials: 'include',
      })
    ).json()
    senseis.value.sort((a, b) => (b < a ? 1 : -1))
    senseis.value.unshift('')

    getRole()
  } catch (e) {
    logging_in.value = 0
  }

  await fetchCurrent()

  async function fetchCurrent() {
    try {
      var data = await fetch('/api/students', {
        credentials: 'include',
      })
      students.value = ((await data.json()) as Student[]).sort((a: Student, b: Student) => {
        if (!a.time && b.time) return 1
        if (a.time && !b.time) return -1
        if (a.time > b.time) return 1
        if (a.time < b.time) return -1
        if (a.last_name > b.last_name) return 1
        if (a.last_name < b.last_name) return -1
        if (a.first_name > b.first_name) return 1
        if (a.first_name < b.first_name) return -1
        return 1
      })
    } catch (e) {
      return
    }

    if (!timer) clearTimeout(timer)
    timer = setTimeout(fetchCurrent, 30000)
  }

  async function remove(student: Student, note_type: keyof Student, id: string) {
    if (confirm('Are you sure you want to remove this?')) {
      var list = student[note_type] as Note[]
      list.splice(
        list.findIndex((v) => v.id == id),
        1,
      )
      await fetch(`/api/students/${student.id}/${note_type}`, {
        body: id,
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        method: 'DELETE',
      })
      await fetchCurrent()
    }
  }

  async function add(student: Student, note_type: keyof Student, value: string) {
    if (value.trim() == '') return

    var list = student[note_type] as Note[]
    list.push({ content: value, date: '', id: '', user: '' })
    await fetch(`/api/students/${student.id}/${note_type}`, {
      body: JSON.stringify({ note: value }),
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      method: 'PUT',
    })
    await fetchCurrent()
  }

  async function edit(student: Student, note_type: keyof Student, value: string) {
    var list = student[note_type] as Note[]
    var note = list.find((val) => val.id == value)
    console.log(list)
    console.log(note)
    if (note !== undefined) {
      let new_value = prompt('Edit note', note.content)
      if (new_value) {
        await fetch(`/api/students/${student.id}/${note_type}`, {
          body: JSON.stringify({ id: value, note: new_value }),
          headers: { 'Content-Type': 'application/json' },
          credentials: 'include',
          method: 'PATCH',
        })
        await fetchCurrent()
      }
    }
  }

  async function setAssigned(student: Student, to: string) {
    await fetch(`/api/students/${student.id}/assigned`, {
      body: JSON.stringify({ note: to }),
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      method: 'PUT',
    })
    await fetchCurrent()
  }

  async function changePass() {
    let new_pass = prompt('Enter a new password')
    if (new_pass !== null) {
      await fetch('/api/change_pass', {
        body: `{"new_pass":"${new_pass}"}`,
        headers: { ['Content-Type']: 'application/json' },
        method: 'POST',
      })
    }
  }

  async function login() {
    logging_in.value = 1
    var username_value = username.value
    var secret_value = secret.value
    username.value = ''
    secret.value = ''
    var res = await fetch('/api/login', {
      body: JSON.stringify({ name: username_value, pass: secret_value }),
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      method: 'POST',
    })
    if (res.status != 200) {
      logging_in.value = 0
      return
    }
    await fetchCurrent()
    logging_in.value = 2
    $cookies?.set('name', username_value)

    getRole()
  }

  async function getRole() {
    try {
      role.value = await (await fetch('/api/role')).json()
    } catch (e) {
      console.warn(e)
    }
  }

  async function logout() {
    logging_in.value = 3
    await fetch('/api/logout', {
      credentials: 'include',
      method: 'POST',
    })
    students.value = []
    logging_in.value = 0
    $cookies?.remove('name')

    role.value = ''
  }

  async function saveFilter() {
    localStorage.setItem('filter_by', filter_by.value)
  }
</script>

<template>
  <div style="display: flex; flex-direction: column; gap: 8px">
    <div style="display: block">
      <div style="display: flex; flex-direction: row; gap: 6px; float: left">
        <input
          v-if="logging_in == 0"
          type="text"
          name="username"
          id="username"
          v-model="username"
          @keydown.enter="
            login()
            ;($event.target as HTMLInputElement).blur()
          "
          autofocus
        />
        <input
          v-if="logging_in == 0"
          type="password"
          name="secret"
          id="secret"
          v-model="secret"
          @keydown.enter="
            login()
            ;($event.target as HTMLInputElement).blur()
          "
        />
        <button v-if="logging_in == 0" @click="login()">Login</button>
        <p v-if="logging_in == 1">Logging in...</p>
        <button v-if="logging_in == 2" @click="logout()">Logout</button>
        <button v-if="logging_in == 2" @click="changePass()">Change Password</button>
        <p v-if="logging_in == 3">Logging out...</p>
        <a v-if="role == 'Admin'" href="/admin">Manage</a>
      </div>
      <div class="right-header" style="display: flex; flex-direction: row; gap: 6px; float: right">
        <p>There are {{ students.filter((v) => v.date).length }} students in today.</p>
        <div
          style="display: block; align-self: stretch; width: 1px; border: 1px solid lightgray"
        ></div>
        <label for="filter-by">Filter by:</label>
        <select id="filter-by" v-model="filter_by" @change="saveFilter()">
          <option value="today">Today</option>
          <option value="for_you">For you</option>
          <option value="all">All</option>
        </select>
      </div>
    </div>
    <div class="student-grid">
      <p style="grid-column: 1; font-weight: bold">Name</p>
      <p style="grid-column: 2; font-weight: bold">Belt</p>
      <p style="grid-column: 3; font-weight: bold">Time</p>
      <p style="grid-column: 4; font-weight: bold">Logins</p>
      <p style="grid-column: 5; font-weight: bold">Notes</p>
      <p style="grid-column: 6; font-weight: bold">Behaviours</p>
      <p style="grid-column: 7; font-weight: bold">Assigned</p>
      <template v-if="students">
        <template v-for="student in students">
          <StudentRow
            v-if="
              (filter_by == 'today' && student.date) ||
              (filter_by == 'for_you' &&
                student.date &&
                (student.assigned == $cookies.get('name') || (student.assigned ?? '') == '')) ||
              filter_by == 'all'
            "
            :key="student.name"
            :name="student.name"
            :belt="student.belt"
            :time="student.time"
            :logins="student.logins"
            :notes="student.notes"
            :behaviours="student.behaviours"
            :assigned="student.assigned"
            :assignable_to="senseis"
            @remove-login="remove(student, 'logins', $event)"
            @remove-note="remove(student, 'notes', $event)"
            @remove-behaviour="remove(student, 'behaviours', $event)"
            @add-login="add(student, 'logins', $event)"
            @add-note="add(student, 'notes', $event)"
            @add-behaviour="add(student, 'behaviours', $event)"
            @set-assigned="setAssigned(student, $event)"
            @edit-login="edit(student, 'logins', $event)"
            @edit-note="edit(student, 'notes', $event)"
            @edit-behaviour="edit(student, 'behaviours', $event)"
          />
        </template>
      </template>
    </div>
  </div>
</template>

<style lang="scss">
  .student-grid {
    display: grid;
    flex-grow: 1;
    grid-template-columns: min-content min-content min-content min-content auto min-content min-content;
    gap: 0;

    > * {
      padding: 3px;
      border: 1px solid grey;
    }
  }

  .right-header p {
    margin: auto 0px;
  }
</style>
