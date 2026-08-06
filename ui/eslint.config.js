import js from '@eslint/js';
import ts from 'typescript-eslint';
import reactPlugin from 'eslint-plugin-react';
import reactHooksPlugin from 'eslint-plugin-react-hooks';
import jsxA11yPlugin from 'eslint-plugin-jsx-a11y';
import reactRefreshPlugin from 'eslint-plugin-react-refresh';

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  reactPlugin.configs.flat.recommended,
  reactPlugin.configs.flat['jsx-runtime'],
  reactHooksPlugin.configs.flat.recommended,
  {
    plugins: {
      'jsx-a11y': jsxA11yPlugin,
    },
    rules: jsxA11yPlugin.configs.recommended.rules,
  },
  {
    plugins: {
      'react-refresh': reactRefreshPlugin,
    },
    rules: {
      'react-refresh/only-export-components': [
        'warn',
        { allowConstantExport: true },
      ],
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/consistent-type-imports': 'warn',
      // New react-hooks v7 strict rules — pre-existing patterns, suppress for now
      'react-hooks/set-state-in-effect': 'off',
      'react-hooks/refs': 'off',
      'react-hooks/immutability': 'off',
      'react-hooks/preserve-manual-memoization': 'off',
      'react-hooks/purity': 'off',
      '@typescript-eslint/no-empty-object-type': 'off',
    },
    settings: {
      react: { version: 'detect' },
    },
  },
  {
    files: ['vite.config.js'],
    rules: {
      'no-undef': 'off',
    },
  },
  {
    // This standalone Playwright probe runs in a Node process but evaluates
    // browser globals inside page.evaluate callbacks.
    files: ['e2e/probe-ws.mjs'],
    languageOptions: {
      globals: {
        console: 'readonly',
        document: 'readonly',
        getComputedStyle: 'readonly',
        matchMedia: 'readonly',
        process: 'readonly',
      },
    },
  },
  {
    // ESLint flat config does NOT auto-respect .gitignore — every build
    // output dir must be listed here explicitly or `eslint .` lints the
    // minified bundles (see ui/.gitignore for the matching set).
    ignores: [
      'dist',
      'dist-ssr',
      'dist-tablet',
      'node_modules',
      'coverage',
      'playwright-report',
      'e2e-results',
      'test-results',
      '.vite',
      '.eslintcache',
    ],
  },
);
