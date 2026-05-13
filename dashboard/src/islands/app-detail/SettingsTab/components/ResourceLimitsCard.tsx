import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  memoryMb: string;
  cpuShares: string;
  onMemoryMbChange: (v: string) => void;
  onCpuSharesChange: (v: string) => void;
};

export default function ResourceLimitsCard({
  memoryMb,
  cpuShares,
  onMemoryMbChange,
  onCpuSharesChange,
}: Props) {
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>Resource Limits</h2>
      {!memoryMb && !cpuShares && (
        <div class={styles.warningBanner} role="alert">
          No resource limits configured. A runaway process could consume all server resources.
        </div>
      )}
      <div class={formStyles.fieldRow}>
        <div>
          <label htmlFor="settings-memory" class={formStyles.label}>Memory Limit (MB)</label>
          <input
            id="settings-memory"
            class={formStyles.input}
            type="number"
            min="64"
            placeholder="No limit"
            value={memoryMb}
            onInput={(e) => onMemoryMbChange((e.target as HTMLInputElement).value)}
          />
          <span class={styles.fieldHint}>Minimum 64 MB. Leave empty for no limit.</span>
        </div>
        <div>
          <label htmlFor="settings-cpu" class={formStyles.label}>CPU Shares</label>
          <input
            id="settings-cpu"
            class={formStyles.input}
            type="number"
            min="1"
            placeholder="1024 (default)"
            value={cpuShares}
            onInput={(e) => onCpuSharesChange((e.target as HTMLInputElement).value)}
          />
          <span class={styles.fieldHint}>Relative weight. Default is 1024. Higher = more CPU time.</span>
        </div>
      </div>
      <p class={styles.settingsNote}>Changes take effect on next deployment.</p>
    </div>
  );
}
