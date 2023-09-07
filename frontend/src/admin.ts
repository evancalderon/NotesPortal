import { createApp, Suspense } from 'vue'
import AdminPage from './AdminPage.vue'
import VueCookies from 'vue-cookies'
import './assets/main.css'

createApp(AdminPage).use(VueCookies).mount('#admin')
