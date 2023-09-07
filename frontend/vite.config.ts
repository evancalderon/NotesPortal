import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue(), vueJsx()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  publicDir: './static',
  build: {
    emptyOutDir: true,
    outDir: '../web',
    assetsDir: 'assets',
    rollupOptions: {
      input: ['index.html', 'admin.html', 'create_user.html'],
    },
  },
  server: {
    proxy: {
      '^/api': {
        target: 'http://10.66.0.5:12000/',
        changeOrigin: false,
        secure: false,
      },
    },
  },
})
