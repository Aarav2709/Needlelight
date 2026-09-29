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
      // An additional websocket connect-src is required for Vite dev tools to work
      if (directive === 'connect-src') {
        values = [...values, 'ws://localhost:1420']
      }

      return `${directive} ${values.join(' ')}`
    })
    .join('; ')
}

// https://vitejs.dev/config/
export default defineConfig({
  assetsInclude: ['**/*.gltf'],
  css: {
    preprocessorOptions: {
      scss: {
        // TODO: dont forget about this
        silenceDeprecations: ['import'],
      },
    },
  },
  resolve: {
    // One copy of Vue for everything. (There used to also be an alias pointing `vue` at the raw
    // runtime file; that bypassed pre-bundling and could load a second, separate runtime.)
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

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  // prevent vite from obscuring rust errors
  clearScreen: false,
  // tauri expects a fixed port, fail if that port is not available
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
  // to make use of `TAURI_ENV_DEBUG` and other env variables
  // https://v2.tauri.app/reference/environment-variables/#tauri-cli-hook-commands
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // Tauri supports es2021
    target: process.env.TAURI_ENV_PLATFORM == 'windows' ? 'chrome105' : 'safari13', // eslint-disable-line turbo/no-undeclared-env-vars
    // don't minify for debug builds
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false, // eslint-disable-line turbo/no-undeclared-env-vars
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_ENV_DEBUG, // eslint-disable-line turbo/no-undeclared-env-vars
    commonjsOptions: {
      esmExternals: true,
    },
  },
  optimizeDeps: {
    entries: ['index.html'],
    // No `exclude` for the @modrinth/* packages: they're aliased to local source (outside
    // node_modules), so Vite already treats them as source code and never pre-bundles them.
    // Excluding them only stopped the startup scan from seeing their dependencies, which were
    // then discovered mid-session, forcing repeated re-optimizations + reloads that left two
    // copies of Vue's runtime loaded ("resolveComponent can only be used in render() or setup()").
    include: [
      'dayjs',
      'dayjs/plugin/duration',
      'dayjs/plugin/isToday',
      'dayjs/plugin/isYesterday',
      'dayjs/plugin/relativeTime',
    ],
  },
})
