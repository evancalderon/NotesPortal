import { createApp } from 'vue'
import App from './App.vue'
import VueCookies from 'vue-cookies'
import './assets/main.css'

createApp(App).use(VueCookies).mount('#app')
