import { defineConfig } from 'astro/config';
import preact from '@astrojs/preact';
import node from '@astrojs/node';
import path from 'path';

export default defineConfig({
  integrations: [preact()],
  devToolbar: { enabled: false },
  output: 'static',
  adapter: node({ mode: 'standalone' }),
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
