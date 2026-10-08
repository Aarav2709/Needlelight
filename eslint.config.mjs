// lints the app's own code, the vendored modrinth packages and the rust backend have their own tooling
import js from '@eslint/js'
import simpleImportSort from 'eslint-plugin-simple-import-sort'
import pluginVue from 'eslint-plugin-vue'
import globals from 'globals'
import tseslint from 'typescript-eslint'

export default tseslint.config(
	{ ignores: ['dist/**', 'src-tauri/**', 'packages/**', 'node_modules/**'] },
	js.configs.recommended,
	...tseslint.configs.recommended,
	...pluginVue.configs['flat/recommended'],
	{
		files: ['**/*.vue'],
		languageOptions: {
			parserOptions: { parser: tseslint.parser, extraFileExtensions: ['.vue'] },
		},
	},
	{
		languageOptions: {
			ecmaVersion: 'latest',
			sourceType: 'module',
			globals: { ...globals.browser },
		},
		plugins: { 'simple-import-sort': simpleImportSort },
		rules: {
			'simple-import-sort/imports': 'error',
			'simple-import-sort/exports': 'error',
			'vue/multi-word-component-names': 'off',
			// layout and wrapping are left to the formatter
			'vue/html-self-closing': 'off',
			'vue/max-attributes-per-line': 'off',
			'vue/singleline-html-element-content-newline': 'off',
			'vue/multiline-html-element-content-newline': 'off',
			'vue/html-indent': 'off',
			'vue/html-closing-bracket-newline': 'off',
			'vue/first-attribute-linebreak': 'off',
			'vue/html-closing-bracket-spacing': 'off',
		},
	},
)
