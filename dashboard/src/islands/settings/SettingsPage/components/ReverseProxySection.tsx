import { useState, useEffect } from 'preact/hooks';
import { RefreshCw, Plus, Trash2, Network, Save } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { GlobalProxySettings } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import Toggle from '@islands/shared/Toggle/Toggle';
import Badge from '@islands/shared/Badge/Badge';
import CodeBlock from '@islands/shared/CodeBlock/CodeBlock';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';
import proxyStyles from './reverse-proxy-section.module.css';

type Props = {
  onSaveMessage: (msg: string) => void;
};

type HeaderEntry = { name: string; value: string };

function parseHeaders(json: string | null): HeaderEntry[] {
  if (!json) return [];
  try {
    const obj = JSON.parse(json) as Record<string, string>;
    return Object.entries(obj).map(([name, value]) => ({ name, value: String(value) }));
  } catch {
    return [];
  }
}

export default function ReverseProxySection({ onSaveMessage }: Props) {
  const [settings, setSettings] = useState<GlobalProxySettings | null>(null);
  const [forceHttps, setForceHttps] = useState(true);
  const [headers, setHeaders] = useState<HeaderEntry[]>([]);
  const [rateRequests, setRateRequests] = useState('');
  const [rateWindow, setRateWindow] = useState<'minute' | 'second'>('minute');
  const [portRangeStart, setPortRangeStart] = useState('10000');
  const [portRangeEnd, setPortRangeEnd] = useState('10100');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [fullConfig, setFullConfig] = useState<string | null>(null);

  useEffect(() => {
    api.getGlobalProxySettings()
      .then(({ data }) => {
        setSettings(data);
        setForceHttps(data.force_https);
        setPortRangeStart(String(data.public_port_range_start));
        setPortRangeEnd(String(data.public_port_range_end));
        setHeaders(parseHeaders(data.default_headers));
        try {
          const rl = data.default_rate_limit ? JSON.parse(data.default_rate_limit) : null;
          if (rl) { setRateRequests(String(rl.requests ?? '')); setRateWindow(rl.window ?? 'minute'); }
        } catch { /* ignore malformed stored value */ }
      })
      .catch(() => setSettings(null))
      .finally(() => setLoading(false));
  }, []);

  const save = async () => {
    setSaving(true);
    try {
      const headerObj: Record<string, string> = {};
      for (const h of headers) if (h.name.trim()) headerObj[h.name.trim()] = h.value;
      const rateLimit = rateRequests
        ? { enabled: true, requests: Number(rateRequests) || 0, window: rateWindow, burst: 0 }
        : null;

      const start = Number(portRangeStart);
      const end = Number(portRangeEnd);
      if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1024 || end > 65535) {
        addToast('error', 'Public port range must be whole numbers between 1024 and 65535.');
        setSaving(false);
        return;
      }
      if (start > end) {
        addToast('error', 'Public port range start must not exceed the end.');
        setSaving(false);
        return;
      }

      const { data } = await api.updateGlobalProxySettings({
        default_headers: Object.keys(headerObj).length ? headerObj : null,
        default_rate_limit: rateLimit,
        force_https: forceHttps,
        public_port_range_start: start,
        public_port_range_end: end,
      });
      setSettings(data);
      onSaveMessage('Reverse proxy settings saved');
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Failed to save proxy settings');
    } finally {
      setSaving(false);
    }
  };

  const reload = async () => {
    setReloading(true);
    try {
      const { message } = await api.reloadProxy();
      addToast('success', message);
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Reload failed');
    } finally {
      setReloading(false);
    }
  };

  const viewFullConfig = async () => {
    if (fullConfig !== null) { setFullConfig(null); return; }
    try {
      const { data } = await api.getFullProxyConfig();
      setFullConfig(JSON.stringify(data ?? {}, null, 2));
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Could not load config');
    }
  };

  if (loading) {
    return (
      <div class={styles.section}>
        <h2 class={styles.sectionHeading}><Network size={18} aria-hidden="true" /> Reverse Proxy</h2>
        <p class={styles.hint} style={{ marginTop: 0 }}>Loading reverse proxy settings…</p>
      </div>
    );
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}><Network size={18} aria-hidden="true" /> Reverse Proxy</h2>
        <span class={proxyStyles.statusPill}>
          {settings?.caddy_running
            ? <Badge variant="success" label="Caddy running" />
            : <Badge variant="error" label="Caddy stopped" />}
          {settings?.caddy_version && <span class={styles.hint}> v{settings.caddy_version}</span>}
        </span>
      </div>
      <p class={styles.hint} style={{ marginTop: 0, marginBottom: 'var(--space-4)' }}>
        Global Caddy defaults applied across every app's routes.
      </p>

      <Toggle
        label="Force HTTPS"
        checked={forceHttps}
        onChange={setForceHttps}
        description="Redirect all HTTP traffic to HTTPS across all apps."
      />

      <h3 class={styles.subHeading} style={{ marginTop: 'var(--space-5)' }}>Global rate limit default</h3>
      <div class={formStyles.fieldRow}>
        <Input
          label="Requests" name="global-rate-requests" type="number" min={0}
          value={rateRequests}
          helpText="Leave blank to disable the global default."
          onChange={setRateRequests}
        />
        <div>
          <label htmlFor="global-rate-window" class={formStyles.label}>Window</label>
          <Select
            id="global-rate-window"
            options={[
              { value: 'minute', label: 'Per minute' },
              { value: 'second', label: 'Per second' },
            ]}
            value={rateWindow}
            onChange={(v) => setRateWindow(v as 'minute' | 'second')}
            fullWidth
          />
        </div>
      </div>

      <h3 class={styles.subHeading} style={{ marginTop: 'var(--space-5)' }}>Public database port range</h3>
      <p class={styles.hint} style={{ marginTop: 0, marginBottom: 'var(--space-3)' }}>
        Ports handed out when a database is exposed for public TCP access. Each public
        database uses one port from this range.
      </p>
      <div class={formStyles.fieldRow}>
        <Input
          label="Range start" name="public-port-range-start" type="number" min={1024} max={65535}
          value={portRangeStart}
          onChange={setPortRangeStart}
        />
        <Input
          label="Range end" name="public-port-range-end" type="number" min={1024} max={65535}
          value={portRangeEnd}
          onChange={setPortRangeEnd}
        />
      </div>

      <h3 class={styles.subHeading} style={{ marginTop: 'var(--space-5)' }}>Global default headers</h3>
      {headers.map((h, i) => (
        <div class={formStyles.fieldRow} key={i} style={{ marginBottom: 'var(--space-3)', alignItems: 'end' }}>
          <Input label="Name" name={`gh-name-${i}`} value={h.name}
            onChange={(v) => setHeaders(headers.map((x, idx) => idx === i ? { ...x, name: v } : x))} />
          <div class={proxyStyles.headerValueRow}>
            <Input label="Value" name={`gh-value-${i}`} value={h.value}
              onChange={(v) => setHeaders(headers.map((x, idx) => idx === i ? { ...x, value: v } : x))} />
            <Button variant="ghost" size="sm" aria-label={`Remove header ${i + 1}`}
              onClick={() => setHeaders(headers.filter((_, idx) => idx !== i))}>
              <Trash2 size={14} aria-hidden="true" />
            </Button>
          </div>
        </div>
      ))}
      <Button variant="secondary" size="sm" onClick={() => setHeaders([...headers, { name: '', value: '' }])}>
        <Plus size={14} aria-hidden="true" /> Add header
      </Button>

      <div class={styles.saveRow} style={{ gap: 'var(--space-3)', justifyContent: 'flex-start' }}>
        <Button variant="primary" onClick={save} loading={saving}>
          <Save size={14} aria-hidden="true" /> Save proxy settings
        </Button>
        <Button variant="secondary" onClick={reload} loading={reloading}>
          <RefreshCw size={14} aria-hidden="true" /> Reload Caddy
        </Button>
        <Button variant="secondary" onClick={viewFullConfig}>
          {fullConfig !== null ? 'Hide full config' : 'View full config'}
        </Button>
      </div>

      {fullConfig !== null && (
        <div style={{ marginTop: 'var(--space-4)' }}>
          <CodeBlock code={fullConfig} language="json" />
        </div>
      )}
    </div>
  );
}
