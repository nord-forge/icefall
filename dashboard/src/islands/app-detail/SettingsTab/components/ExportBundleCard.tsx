import { useState } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { App } from '@lib/types';
import { Package } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from '../settings-tab.module.css';

type Props = {
  app: App;
};

export default function ExportBundleCard({ app }: Props) {
  const [exporting, setExporting] = useState(false);

  async function handleExport() {
    setExporting(true);
    try {
      const { data } = await api.exportBundle(app.id);
      const json = JSON.stringify(data, null, 2);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `${app.name}.icefall.json`;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(url);
      addToast('success', 'Bundle exported successfully');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to export bundle');
    }
    setExporting(false);
  }

  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <Package size={18} aria-hidden="true" /> Portable Bundle
      </h2>
      <p class={styles.settingsDescription}>
        Download this app's configuration as a portable .icefall bundle. The
        bundle includes git settings, build config, environment variables, and
        resource limits. Everything needed to recreate this app on another
        instance.
      </p>
      <div class={styles.saveRow}>
        <Button
          variant="secondary"
          onClick={handleExport}
          loading={exporting}
        >
          <Package size={14} aria-hidden="true" /> Export as bundle
        </Button>
      </div>
      <p class={styles.fieldHint} role="status" aria-live="polite">
        {exporting ? 'Preparing bundle for download...' : ''}
      </p>
    </div>
  );
}
