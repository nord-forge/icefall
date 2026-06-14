import { defineConfig } from 'astro/config';
import preact from '@astrojs/preact';
import path from 'path';

// Fully static build: every page (including the [id]/[token]/[...path]
// dynamic routes) is prerendered to plain HTML. The Rust server serves
// dist/ directly via ServeDir and SPA-falls-back dynamic paths to the
// matching prerendered shell. No Node/SSR adapter is needed — the dynamic
// pages are shells around client:only Preact islands that read their route
// param from window.location.
export default defineConfig({
  integrations: [preact()],
  devToolbar: { enabled: false },
  output: 'static',
  server: { port: 4321 },
  prefetch: {
    prefetchAll: false,
    defaultStrategy: 'hover',
  },
  vite: {
    resolve: {
      alias: {
        '@': path.resolve('./src'),
        '@islands': path.resolve('./src/islands'),
        '@components': path.resolve('./src/components'),
        '@styles': path.resolve('./src/styles'),
        '@lib': path.resolve('./src/lib'),
        '@stores': path.resolve('./src/stores'),
      },
      dedupe: ['preact', 'preact/hooks', 'preact/compat', 'preact/jsx-runtime', '@preact/signals'],
    },
    optimizeDeps: {
      include: ['preact', 'preact/hooks', 'preact/devtools', 'preact/debug', 'preact/jsx-runtime', '@nanostores/preact', 'lucide-preact'],
    },
    css: {
      modules: {
        localsConvention: 'camelCase',
      },
    },
    server: {
      proxy: {
        '/api': {
          target: 'http://localhost:3001',
          changeOrigin: true,
          ws: true,
          configure: (/** @type {any} */ proxy) => {
            proxy.on('proxyRes', (/** @type {any} */ proxyRes) => {
              if (proxyRes.headers['content-type']?.includes('text/event-stream')) {
                proxyRes.headers['cache-control'] = 'no-cache';
                proxyRes.headers['x-accel-buffering'] = 'no';
              }
            });
          },
        },
      },
    },
  },
});
