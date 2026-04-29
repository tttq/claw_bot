import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const host = process.env.TAURI_DEV_HOST

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:1421',
        changeOrigin: true,
        secure: false,
        ws: true,
        configure: (proxy, _options) => {
          proxy.on('error', (err, _req, res) => {
            console.log('[Proxy] ❌ Backend connection error:', err.message)
            if (res.writeHead && res.end) {
              res.writeHead(502, { 'Content-Type': 'application/json' })
              res.end(JSON.stringify({ success: false, error: 'Backend not running on port 1421' }))
            }
          })
          proxy.on('proxyReq', (_proxyReq, req, _res) => {
            console.log('[Proxy] ➡️', req.method, req.url)
          })
          proxy.on('proxyRes', (proxyRes, req, _res) => {
            console.log('[Proxy] ✅', req.method, req.url, '→', proxyRes.statusCode)
          })
        },
      },
    },
  },
}))