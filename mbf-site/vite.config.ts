import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import viteTsconfigPaths from 'vite-tsconfig-paths'
import mkcert from 'vite-plugin-mkcert'
import legacy from '@vitejs/plugin-legacy'
//@ts-ignore
import type { PluginObj } from '@babel/core'
import babel from 'vite-plugin-babel'

function babelTransformBigInt(): PluginObj {
    return {
        visitor: {
            BigIntLiteral(path: any) {
                const value = path.node.value // string representation of the number
                path.replaceWithSourceString(`BigInt("${value}")`)
            },
        },
    }
}

export default defineConfig({
    base: process.env.BASE_URL ?? './',
    plugins: [
                
        react(), 
        viteTsconfigPaths(), 
        mkcert(),
babel({
            babelConfig: {
                plugins: [babelTransformBigInt],
            }
        }),
        legacy({ 
            targets: ['chrome >= 74']
        }), 
    ],
    server: {
        open: process.env.BROWSER='chrome',
        port: 3000,
        https: true,
    },
    preview: {
        port: 3000,
        https: true,
    },
    build: {
        sourcemap: true
    },
    resolve: {
    }
});
