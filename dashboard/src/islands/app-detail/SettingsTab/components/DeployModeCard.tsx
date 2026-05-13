import type { DeployMode } from '@lib/types';
import Select from '@islands/shared/Select/Select';
import { Zap } from 'lucide-preact';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  deployMode: DeployMode;
  onDeployModeChange: (v: DeployMode) => void;
};

export default function DeployModeCard({ deployMode, onDeployModeChange }: Props) {
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <Zap size={18} /> Deploy Mode
      </h2>
      <p class={styles.settingsDescription}>
        Controls how this app is built and served. Native mode builds on the host and serves static files directly through Caddy without container overhead. Container mode uses Docker for apps that need a running server.
      </p>
      <div class={formStyles.fieldRow}>
        <div>
          {/* a11y [WCAG 4.1.2]: select has associated label via htmlFor/id */}
          <label htmlFor="settings-deploy-mode" class={formStyles.label}>Deploy Mode</label>
          <Select
            id="settings-deploy-mode"
            options={[
              { value: 'auto', label: 'Auto (recommended)' },
              { value: 'native', label: 'Native (static)' },
              { value: 'container', label: 'Container' },
            ]}
            value={deployMode}
            onChange={(v) => onDeployModeChange(v as DeployMode)}
            fullWidth
          />
          <span class={styles.fieldHint}>
            {deployMode === 'auto' && 'Icefall detects whether your app is static or needs a server, then picks the fastest deploy method.'}
            {deployMode === 'native' && 'Build output is served directly by Caddy. Best for static sites (React, Vue, Astro static, plain HTML). No container overhead.'}
            {deployMode === 'container' && 'App runs in a Docker container. Required for SSR frameworks (Next.js, Nuxt, Astro SSR) and Node.js servers.'}
          </span>
        </div>
      </div>
      <p class={styles.settingsNote}>Changes take effect on next deployment.</p>
    </div>
  );
}
