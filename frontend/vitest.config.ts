import { defineConfig } from 'vitest/config';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte({ preprocess: vitePreprocess() })],
  test: {
    environment: 'jsdom',
    include: ['app/tests/**/*.spec.ts', 'app/tests/**/*.test.ts'],
  }
});
