/* eslint-env node */
module.exports = {
  root: true,
  env: {
    browser: true,
    es2022: true,
    node: true,
  },
  extends: [
    'plugin:vue/vue3-recommended',
    'eslint:recommended',
    '@vue/eslint-config-typescript',
    '@vue/eslint-config-prettier/skip-formatting',
  ],
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
  },
  rules: {
    // The wallet bridge and RPC payloads are dynamically typed; `any` is pervasive
    // and out of scope for a convention pass. Keep it visible but non-blocking.
    '@typescript-eslint/no-explicit-any': 'off',
    // Allow intentionally-unused args/vars when prefixed with `_`.
    '@typescript-eslint/no-unused-vars': [
      'warn',
      { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrors: 'none' },
    ],
    // Single-file views/components use descriptive multi-word names already; the
    // few single-word route views (Layout, Logo) are intentional.
    'vue/multi-word-component-names': 'off',
    'no-empty': ['warn', { allowEmptyCatch: true }],
    'no-constant-condition': ['error', { checkLoops: false }],
  },
}
