import type { App } from '@lib/types';
import { FileText } from 'lucide-preact';
import NoisePatternConfig from '@islands/logs/LogViewer/components/NoisePatternConfig';
import styles from '../settings-tab.module.css';

type Props = {
  app: App;
};

export default function LogPatternsCard({ app }: Props) {
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <FileText size={18} aria-hidden="true" /> Log Patterns
      </h2>
      <p class={styles.settingsDescription}>
        Configure noise and highlight patterns for this application's log viewer.
        Noise patterns hide routine lines, and highlight patterns emphasize important ones.
      </p>
      <NoisePatternConfig
        appId={app.id}
        noisePatterns={app.log_noise_patterns || ''}
        highlightPatterns={app.log_highlight_patterns || ''}
      />
    </div>
  );
}
