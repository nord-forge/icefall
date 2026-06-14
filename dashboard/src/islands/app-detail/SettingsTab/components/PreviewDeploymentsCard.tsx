import Input from '@islands/shared/Input/Input';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  previewEnabled: boolean;
  previewBranchPattern: string;
  onPreviewEnabledChange: (v: boolean) => void;
  onPreviewBranchPatternChange: (v: string) => void;
};

export default function PreviewDeploymentsCard({
  previewEnabled,
  previewBranchPattern,
  onPreviewEnabledChange,
  onPreviewBranchPatternChange,
}: Props) {
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>Preview Deployments</h2>
      <p class={styles.settingsDescription}>
        Automatically deploy branches matching a pattern and assign a preview subdomain. Previews are cleaned up when the branch is deleted.
      </p>
      <div class={styles.toggleRow}>
        <label class={styles.toggleLabel}>
          <input
            type="checkbox"
            class={formStyles.checkbox}
            checked={previewEnabled}
            onChange={(e) => onPreviewEnabledChange((e.target as HTMLInputElement).checked)}
          />
          Enable preview deployments
        </label>
      </div>
      {previewEnabled && (
        <div style={{ marginTop: 'var(--space-3)' }}>
          <Input
            label="Branch Pattern"
            name="preview-pattern"
            id="settings-preview-pattern"
            mono
            value={previewBranchPattern}
            onChange={onPreviewBranchPatternChange}
            placeholder="feature/*"
            helpText="Glob pattern. Use * to match all branches except the main deploy branch."
          />
        </div>
      )}
    </div>
  );
}
