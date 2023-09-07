import { createApp, Suspense } from 'vue'
import CreateUserPage from './CreateUserPage.vue'
import VueCookies from 'vue-cookies'
import './assets/main.css'

createApp(CreateUserPage).use(VueCookies).mount('#admin')
