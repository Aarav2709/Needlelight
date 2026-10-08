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

// modules whose top level code is setup only their own exports need, so rollup may drop them when unused
const LAZY_ONLY_MODULES =
  /packages\/utils\/(parse\.ts|highlightjs\/)|node_modules\/(markdown-it|highlight\.js|highlightjs-mcfunction|xss|cssfilter|entities|linkify-it|mdurl|punycode\.js|uc\.micro)\//

// vite config
export default defineConfig({
  assetsInclude: ['**/*.gltf'],
  // vue i18n feature flags, the modrinth ui compiles messages itself and only the composition api is used
  define: {
    __VUE_I18N_FULL_INSTALL__: false,
    __VUE_I18N_LEGACY_API__: false,
    __INTLIFY_DROP_MESSAGE_COMPILER__: true,
    __INTLIFY_PROD_DEVTOOLS__: false,
  },
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
    target: process.env.TAURI_ENV_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    // don't minify debug builds
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    // sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    commonjsOptions: {
      esmExternals: true,
    },
    rollupOptions: {
      treeshake: {
        // markdown and syntax highlighting only load where a readme is shown, not at startup
        moduleSideEffects: (id) => !LAZY_ONLY_MODULES.test(id.replace(/\\/g, '/')),
      },
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
