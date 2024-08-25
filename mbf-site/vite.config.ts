import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import viteTsconfigPaths from 'vite-tsconfig-paths'
import mkcert from 'vite-plugin-mkcert'

export default defineConfig({
    base: process.env.BASE_URL ?? '/',
    plugins: [react(), viteTsconfigPaths(), mkcert()],
    server: {    
        open: true,
        port: 3000,
        https: true
    },
    build: {
        sourcemap: true
    }
})