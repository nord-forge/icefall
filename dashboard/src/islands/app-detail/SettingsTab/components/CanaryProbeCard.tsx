import { useState, useEffect } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { App } from '@lib/types';
import { FlaskConical } from 'lucide-preact';
import Toggle from '@islands/shared/Toggle/Toggle';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  app: App;
};

type CanaryConfig = {
  url: string;
  request_count: number;
  error_threshold: number;
  latency_threshold_ms: number;
};

function parseConfig(raw: string | null): CanaryConfig {
  if (!raw) {
    return {
      url: '',
      request_count: 50,
      error_threshold: 5,
      latency_threshold_ms: 500,
    };
  }
  try {
    return JSON.parse(raw);
  } catch {
    return {
      url: '',
      request_count: 50,
      error_threshold: 5,
      latency_threshold_ms: 500,
    };
  }
}

export default function CanaryProbeCard({ app }: Props) {
  const [enabled, setEnabled] = useState(app.canary_enabled);
  const [config, setConfig] = useState<CanaryConfig>(() =>
    parseConfig(app.canary_config)
  );
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setEnabled(app.canary_enabled);
    setConfig(parseConfig(app.canary_config));
  }, [app.canary_enabled, app.canary_config]);

  async function handleSave() {
    setSaving(true);
    try {
      await api.updateApp(app.id, {
        canary_enabled: enabled,
        canary_config: JSON.stringify(config),
      });
      addToast('success', 'Canary probe settings saved');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to save canary settings');
    }
    setSaving(false);
  }

  async function handleToggle(checked: boolean) {
    setEnabled(checked);
    setSaving(true);
    try {
      await api.updateApp(app.id, {
        canary_enabled: checked,
        canary_config: JSON.stringify(config),
      });
      addToast('success', checked ? 'Canary probe enabled' : 'Canary probe disabled');
    } catch (err: any) {
      setEnabled(!checked);
      addToast('error', err.message || 'Failed to update canary settings');
    }
    setSaving(false);
  }

  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <FlaskConical size={18} aria-hidden="true" /> Canary Probe
      </h2>
      <p class={styles.settingsDescription}>
        Run a canary health check against new deployments before routing
        production traffic. If the canary fails, the deployment is automatically
        rolled back.
      </p>

      <Toggle
        label="Enable canary probe"
        checked={enabled}
        onChange={handleToggle}
      />

      {enabled && (
        <div style={{ marginTop: 'var(--space-4)' }}>
          <div class={formStyles.fieldRow}>
            <Input
              label="Health check URL"
              name="canary-url"
              id="canary-url"
              type="url"
              value={config.url}
              onChange={(v) => setConfig((c) => ({ ...c, url: v }))}
              placeholder="/health"
              helpText="Relative path or full URL to check during canary."
            />
            <Input
              label="Request count"
              name="canary-requests"
              id="canary-requests"
              type="number"
              min={1}
              value={String(config.request_count)}
              onChange={(v) => setConfig((c) => ({ ...c, request_count: parseInt(v, 10) || 0 }))}
              helpText="Number of requests to send during the canary phase."
            />
          </div>
          <div class={formStyles.fieldRow} style={{ marginTop: 'var(--space-4)' }}>
            <Input
              label="Error threshold (%)"
              name="canary-error"
              id="canary-error"
              type="number"
              min={0}
              max={100}
              value={String(config.error_threshold)}
              onChange={(v) => setConfig((c) => ({ ...c, error_threshold: parseFloat(v) || 0 }))}
              helpText="Maximum acceptable error rate before failing the canary."
            />
            <Input
              label="Latency threshold (ms)"
              name="canary-latency"
              id="canary-latency"
              type="number"
              min={0}
              value={String(config.latency_threshold_ms)}
              onChange={(v) => setConfig((c) => ({ ...c, latency_threshold_ms: parseInt(v, 10) || 0 }))}
              helpText="p95 latency above this value fails the canary."
            />
          </div>
          <div class={styles.saveRow} style={{ marginTop: 'var(--space-4)' }}>
            <Button
              variant="primary"
              onClick={handleSave}
              loading={saving}
            >
              Save canary settings
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
