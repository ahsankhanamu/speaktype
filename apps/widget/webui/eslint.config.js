import svelte from 'eslint-plugin-svelte';
import ts from 'typescript-eslint';

const rules = {
  'no-restricted-imports': ['error', {
    paths: [
      {
        name: '@tauri-apps/api/event',
        message: 'Use useEvent or liveQuery from the data layer.',
      },
      {
        name: '$lib/bindings',
        importNames: ['events'],
        message: 'Use useEvent or liveQuery from the data layer.',
      },
    ],
  }],
  'no-restricted-syntax': ['error', {
    selector: 'CallExpression[callee.name="setInterval"], CallExpression[callee.property.name="setInterval"]',
    message: 'use liveQuery({ pollMs })',
  }],
};

export default [
  {
    // svelte-eslint-parser 1.10.1 rejects this valid file's parenthesized @const ternaries.
    ignores: ['dist/**', 'src/lib/bindings.ts', 'src/settings/DebugTab.svelte'],
  },
  ...svelte.configs['flat/base'],
  {
    files: ['src/**/*.{js,ts}'],
    languageOptions: {
      parser: ts.parser,
      parserOptions: { ecmaVersion: 'latest', sourceType: 'module' },
    },
    rules,
  },
  {
    files: ['src/**/*.svelte'],
    plugins: { svelte },
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    rules,
  },
  {
    // These pre-existing polling loops belong to non-F5 migration work.
    files: [
      'src/onboarding/Onboarding.svelte',
      'src/settings/Settings.svelte',
      'src/widget/Widget.svelte',
    ],
    rules: { 'no-restricted-syntax': 'off' },
  },
  {
    files: ['src/lib/data/**/*.ts'],
    rules: {
      'no-restricted-imports': 'off',
      'no-restricted-syntax': 'off',
    },
  },
];
