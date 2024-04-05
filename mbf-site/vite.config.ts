import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import viteTsconfigPaths from 'vite-tsconfig-paths'

export default defineConfig({
    base: process.env.BASE_URL ?? '/',
    plugins: [react(), viteTsconfigPaths()],
    server: {    
        open: true,
        port: 3000,
    }
})