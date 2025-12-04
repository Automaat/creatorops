import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  // @ts-expect-error vite/vitest plugin type mismatch
  plugins: [react()],
  test: {
    coverage: {
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.test.{ts,tsx}',
        '**/*.spec.{ts,tsx}',
        'src-tauri/',
        'dist/',
        'vite.config.ts',
        'vitest.config.ts',
        'eslint.config.js',
      ], provider: 'v8', reporter: ['text', 'json', 'html', 'lcov'],
    }, environment: 'jsdom', globals: true, setupFiles: './src/test/setup.ts',
  },
})
