import { useState } from 'preact/hooks';
import type { App } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Toggle from '@islands/shared/Toggle/Toggle';
import { Shield } from 'lucide-preact';
import styles from '../settings-tab.module.css';

type Props = {
  app: App;
};

export default function TunnelCard({ app }: Props) {
  const [enabled, setEnabled] = useState(app.tunnel_enabled);
  const [saving, setSaving] = useState(false);

  async function handleToggle(checked: boolean) {
    setSaving(true);
    try {
      await api.updateApp(app.id, { tunnel_enabled: checked });
      setEnabled(checked);
      addToast('success', checked ? 'Tunnel routing enabled' : 'Tunnel routing disabled');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to update tunnel setting');
    }
    setSaving(false);
  }

  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <Shield size={18} aria-hidden="true" /> Cloudflare Tunnel
      </h2>
      <p class={styles.settingsDescription}>
        Route traffic through Cloudflare Tunnel for enhanced security and DDoS protection. Requires a tunnel to be configured in global settings.
      </p>
      <Toggle
        label="Enable tunnel routing"
        description="All traffic to this app will be routed through the configured Cloudflare Tunnel"
        checked={enabled}
        disabled={saving}
        onChange={handleToggle}
      />
    </div>
  );
}
