import { useState, useEffect } from 'preact/hooks';
import { RefreshCw, Plus, Trash2 } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { GlobalProxySettings } from '@lib/types';
import Card from '@islands/shared/Card/Card';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Toggle from '@islands/shared/Toggle/Toggle';
import Badge from '@islands/shared/Badge/Badge';
import CodeBlock from '@islands/shared/CodeBlock/CodeBlock';
import styles from './reverse-proxy-section.module.css';

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

  if (loading) return <Card title="Reverse proxy"><p>Loading reverse proxy settings…</p></Card>;

  return (
    <Card title="Reverse proxy">
      <div class={styles.sectionHeader}>
        <p class={styles.muted}>Global Caddy defaults applied across every app's routes.</p>
        <span>
          {settings?.caddy_running
            ? <Badge variant="success" label="Caddy running" />
            : <Badge variant="error" label="Caddy stopped" />}
          {settings?.caddy_version && <span class={styles.muted}> v{settings.caddy_version}</span>}
        </span>
      </div>

      <Toggle
        label="Force HTTPS"
        checked={forceHttps}
        onChange={setForceHttps}
        description="Redirect all HTTP traffic to HTTPS across all apps."
      />

      <fieldset class={styles.fieldset}>
        <legend class={styles.legend}>Global rate limit default</legend>
        <div class={styles.inlineFields}>
          <Input
            label="Requests" name="global-rate-requests" type="number" min={0}
            value={rateRequests}
            helpText="Leave blank to disable the global default."
            onChange={setRateRequests}
          />
          <label class={styles.selectLabel}>
            Window
            <select class={styles.select} value={rateWindow}
              onChange={(e) => setRateWindow((e.target as HTMLSelectElement).value as 'minute' | 'second')}>
              <option value="minute">Per minute</option>
              <option value="second">Per second</option>
            </select>
          </label>
        </div>
      </fieldset>

      <fieldset class={styles.fieldset}>
        <legend class={styles.legend}>Public database port range</legend>
        <p class={styles.muted}>
          Ports handed out when a database is exposed for public TCP access. Each public
          database uses one port from this range.
        </p>
        <div class={styles.inlineFields}>
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
      </fieldset>

      <fieldset class={styles.fieldset}>
        <legend class={styles.legend}>Global default headers</legend>
        {headers.map((h, i) => (
          <div class={styles.inlineFields} key={i}>
            <Input label="Name" name={`gh-name-${i}`} value={h.name}
              onChange={(v) => setHeaders(headers.map((x, idx) => idx === i ? { ...x, name: v } : x))} />
            <Input label="Value" name={`gh-value-${i}`} value={h.value}
              onChange={(v) => setHeaders(headers.map((x, idx) => idx === i ? { ...x, value: v } : x))} />
            <Button variant="ghost" size="sm" aria-label={`Remove header ${i + 1}`}
              onClick={() => setHeaders(headers.filter((_, idx) => idx !== i))}>
              <Trash2 size={14} aria-hidden="true" />
            </Button>
          </div>
        ))}
        <Button variant="secondary" size="sm" onClick={() => setHeaders([...headers, { name: '', value: '' }])}>
          <Plus size={14} aria-hidden="true" /> Add header
        </Button>
      </fieldset>

      <div class={styles.actions}>
        <Button onClick={save} loading={saving}>Save proxy settings</Button>
        <Button variant="secondary" onClick={reload} loading={reloading}>
          <RefreshCw size={14} aria-hidden="true" /> Reload Caddy
        </Button>
        <Button variant="secondary" onClick={viewFullConfig}>
          {fullConfig !== null ? 'Hide full config' : 'View full config'}
        </Button>
      </div>

      {fullConfig !== null && <CodeBlock code={fullConfig} language="json" />}
    </Card>
  );
}
