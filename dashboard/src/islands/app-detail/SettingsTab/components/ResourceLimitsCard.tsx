import Input from '@islands/shared/Input/Input';
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
        <Input
          label="Memory Limit (MB)"
          name="memory"
          id="settings-memory"
          type="number"
          min={64}
          placeholder="No limit"
          value={memoryMb}
          onChange={onMemoryMbChange}
          helpText="Minimum 64 MB. Leave empty for no limit."
        />
        <Input
          label="CPU Shares"
          name="cpu"
          id="settings-cpu"
          type="number"
          min={1}
          placeholder="1024 (default)"
          value={cpuShares}
          onChange={onCpuSharesChange}
          helpText="Relative weight. Default is 1024. Higher = more CPU time."
        />
      </div>
      <p class={styles.settingsNote}>Changes take effect on next deployment.</p>
    </div>
  );
}
