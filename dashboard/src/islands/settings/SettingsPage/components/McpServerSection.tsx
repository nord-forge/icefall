import { Shield } from 'lucide-preact';
import styles from '../settings-page.module.css';

export default function McpServerSection() {
  return (
    <div class={styles.section}>
      <h2 class={styles.sectionHeading}><Shield size={18} aria-hidden="true" /> MCP Server</h2>
      <p class={styles.mcpDescription}>
        Connect AI agents to manage your apps. Add this to your Claude Code settings:
      </p>
      <div class={styles.codeBlock}>
{`{
  "mcpServers": {
    "icefall": {
      "url": "${typeof window !== 'undefined' ? window.location.origin : 'http://localhost:3000'}/api/v1/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_API_TOKEN"
      }
    }
  }
}`}
      </div>
      <p class={styles.hint}>
        Generate an API token on the <a href="/users" class={styles.inlineLink} data-astro-prefetch="hover">Users page</a> to use as the Bearer token.
      </p>
    </div>
  );
}
