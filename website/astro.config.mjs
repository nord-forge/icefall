// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://icefall.dev',
  integrations: [
    starlight({
      title: 'Icefall',
      description: 'A fast, simple, self-hosted deployment platform',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/nordforge/icefall' }],
      logo: { src: './src/assets/logo.svg' },
      customCss: ['./src/styles/custom.css'],
      components: {
        ThemeSelect: './src/components/ThemeSelect.astro',
      },
      editLink: { baseUrl: 'https://github.com/nordforge/icefall/edit/main/website/' },
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Introduction', slug: 'getting-started/introduction' },
            { label: 'Installation', slug: 'getting-started/installation' },
            { label: 'Quick Start', slug: 'getting-started/quick-start' },
          ],
        },
        {
          label: 'Core Concepts',
          items: [
            { label: 'Architecture', slug: 'concepts/architecture' },
            { label: 'How Builds Work', slug: 'concepts/builds' },
            { label: 'Environments', slug: 'concepts/environments' },
            { label: 'Domain Routing', slug: 'concepts/domains' },
          ],
        },
        {
          label: 'Framework Guides',
          items: [
            { label: 'Astro', slug: 'frameworks/astro' },
            { label: 'Next.js', slug: 'frameworks/nextjs' },
            { label: 'React (Vite)', slug: 'frameworks/react' },
            { label: 'Vue', slug: 'frameworks/vue' },
            { label: 'Nuxt', slug: 'frameworks/nuxt' },
            { label: 'Node.js', slug: 'frameworks/nodejs' },
            { label: 'Dockerfile', slug: 'frameworks/dockerfile' },
            { label: 'Static Site', slug: 'frameworks/static' },
          ],
        },
        {
          label: 'Databases',
          items: [
            { label: 'Provisioning', slug: 'databases/provisioning' },
            { label: 'Backups & Restore', slug: 'databases/backups' },
          ],
        },
        {
          label: 'Monitoring',
          items: [
            { label: 'Health Checks', slug: 'monitoring/health-checks' },
            { label: 'Logs', slug: 'monitoring/logs' },
            { label: 'Metrics', slug: 'monitoring/metrics' },
          ],
        },
        {
          label: 'Authentication',
          items: [
            { label: 'Users & Roles', slug: 'auth/users' },
            { label: 'API Tokens', slug: 'auth/tokens' },
            { label: 'OAuth', slug: 'auth/oauth' },
          ],
        },
        {
          label: 'CLI Reference',
          items: [
            { label: 'Commands', slug: 'cli/commands' },
            { label: 'Configuration', slug: 'cli/configuration' },
          ],
        },
        {
          label: 'API Reference',
          items: [
            { label: 'REST API', slug: 'api/rest' },
            { label: 'MCP Server', slug: 'api/mcp' },
          ],
        },
        {
          label: 'Server Management',
          items: [
            { label: 'Migration', slug: 'management/migration' },
            { label: 'Notifications', slug: 'management/notifications' },
            { label: 'Updates', slug: 'management/updates' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Podman Runtime', slug: 'reference/podman' },
          ],
        },
      ],
    }),
  ],
});
