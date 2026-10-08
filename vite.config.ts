import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'
import { defineConfig } from 'vite'
import svgLoader from 'vite-svg-loader'

import tauriConf from './src-tauri/tauri.conf.json'

const projectRootDir = resolve(__dirname)

const buildCspHeader = (csp) => {
  if (!csp) {
    return ''
  }
  if (typeof csp === 'string') {
    return csp
  }

  return Object.entries(csp)
    .map(([directive, sources]) => {
      let values = Array.isArray(sources) ? sources : [sources]
      // vite dev tools need an extra websocket connect source
      if (directive === 'connect-src') {
        values = [...values, 'ws://localhost:1420']
      }

      return `${directive} ${values.join(' ')}`
    })
    .join('; ')
}

// vite config
export default defineConfig({
  assetsInclude: ['**/*.gltf'],
  css: {
    preprocessorOptions: {
      scss: {
        // todo: silences sass import deprecation warnings, remove once the styles stop using import
        silenceDeprecations: ['import'],
      },
    },
  },
  resolve: {
    // one copy of vue for everything so a second runtime never loads
    dedupe: ['vue'],
    alias: [
      {
        find: 'fuse.js/dist/fuse.basic',
        replacement: 'fuse.js/dist/fuse.basic.esm.js',
      },
      {
        find: '@modrinth/assets',
        replacement: resolve(projectRootDir, 'packages/assets'),
      },
      {
        find: '@modrinth/ui',
        replacement: resolve(projectRootDir, 'packages/ui'),
      },
      {
        find: '@modrinth/utils',
        replacement: resolve(projectRootDir, 'packages/utils'),
      },
      {
        find: '@',
        replacement: resolve(projectRootDir, 'src'),
      },
    ],
  },
  plugins: [
    vue(),
    svgLoader({
      svgoConfig: {
        plugins: [
          {
            name: 'preset-default',
            params: {
              overrides: {
                removeViewBox: false,
              },
            },
          },
        ],
      },
    }),
  ],

  // tauri dev options, keep the screen so vite doesn't hide rust errors
  clearScreen: false,
  // tauri expects a fixed port, so fail if it is taken
  server: {
    port: 1420,
    strictPort: true,
    headers: {
      ...(() => {
        const isTauriDev = !!process.env.TAURI_ENV_PLATFORM
        const cspHeader = isTauriDev ? buildCspHeader(tauriConf.app?.security?.csp) : ''
        return cspHeader ? { 'content-security-policy': cspHeader } : {}
      })(),
    },
  },
  // expose the tauri environment variables to the app
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // tauri supports es2021
    target: process.env.TAURI_ENV_PLATFORM == 'windows' ? 'chrome105' : 'safari13', // eslint-disable-line turbo/no-undeclared-env-vars
    // don't minify debug builds
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false, // eslint-disable-line turbo/no-undeclared-env-vars
    // sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_ENV_DEBUG, // eslint-disable-line turbo/no-undeclared-env-vars
    commonjsOptions: {
      esmExternals: true,
    },
  },
  optimizeDeps: {
    entries: ['index.html'],
    // the modrinth packages are aliased to local source, excluding them caused repeated reoptimizing and two vue runtimes
    include: [
      'dayjs',
      'dayjs/plugin/duration',
      'dayjs/plugin/isToday',
      'dayjs/plugin/isYesterday',
      'dayjs/plugin/relativeTime',
    ],
  },
})
