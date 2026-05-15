import { useState, useEffect } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import { Shield, Save, CheckCircle, XCircle } from 'lucide-preact';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  onSaveMessage: (msg: string) => void;
};

export default function CloudflareTunnelSection({ onSaveMessage }: Props) {
  const [tunnelToken, setTunnelToken] = useState('');
  const [tunnelId, setTunnelId] = useState('');
  const [tunnelStatus, setTunnelStatus] = useState<'connected' | 'disconnected' | 'unknown'>('unknown');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetch('/api/v1/settings/tunnel', { credentials: 'same-origin' })
      .then((r) => r.json())
      .then((d) => {
        if (d.data) {
          setTunnelId(d.data.tunnel_id || '');
          setTunnelStatus(d.data.status || 'unknown');
          if (d.data.has_token) setTunnelToken('********');
        }
      })
      .catch(() => {});
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      const body: Record<string, string> = { tunnel_id: tunnelId };
      if (tunnelToken && tunnelToken !== '********') {
        body.tunnel_token = tunnelToken;
      }
      await fetch('/api/v1/settings/tunnel', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'same-origin',
        body: JSON.stringify(body),
      });
      onSaveMessage('Tunnel settings saved');
    } catch {
      onSaveMessage('Failed to save tunnel settings');
    }
    setSaving(false);
  }

  return (
    <div class={styles.section}>
      <h2 class={styles.sectionHeading}>
        <Shield size={18} aria-hidden="true" /> Cloudflare Tunnel
      </h2>
      <div class={formStyles.fieldRow}>
        <Input
          label="Tunnel Token"
          name="cf-tunnel-token"
          id="cf-tunnel-token"
          type="password"
          value={tunnelToken}
          onChange={setTunnelToken}
          placeholder="Enter tunnel token"
          helpText="Found in the Cloudflare Zero Trust dashboard under Networks > Tunnels."
        />
        <Input
          label="Tunnel ID"
          name="cf-tunnel-id"
          id="cf-tunnel-id"
          mono
          value={tunnelId}
          onChange={setTunnelId}
          placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
          helpText="The unique identifier for your Cloudflare Tunnel."
        />
      </div>
      <div class={formStyles.fieldRow}>
        <div>
          <span class={formStyles.label}>Status</span>
          {/* a11y [WCAG 4.1.3]: tunnel status announced to AT */}
          <div role="status" aria-live="polite" style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', marginTop: 'var(--space-1)' }}>
            {tunnelStatus === 'connected' && (
              <>
                <CheckCircle size={16} aria-hidden="true" style={{ color: 'var(--color-success)' }} />
                <span style={{ fontSize: 'var(--text-sm)', color: 'var(--color-success)' }}>Connected</span>
              </>
            )}
            {tunnelStatus === 'disconnected' && (
              <>
                <XCircle size={16} aria-hidden="true" style={{ color: 'var(--color-error)' }} />
                <span style={{ fontSize: 'var(--text-sm)', color: 'var(--color-error)' }}>Disconnected</span>
              </>
            )}
            {tunnelStatus === 'unknown' && (
              <span style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)' }}>Not configured</span>
            )}
          </div>
        </div>
      </div>
      <div class={styles.saveRow}>
        <Button variant="primary" onClick={handleSave} loading={saving}>
          <Save size={14} aria-hidden="true" /> Save Tunnel
        </Button>
      </div>
    </div>
  );
}
