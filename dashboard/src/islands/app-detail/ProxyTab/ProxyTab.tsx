import { useEffect, useState } from 'preact/hooks';
import { Copy, Plus, Trash2, ShieldAlert } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { ProxyConfig, ProxyPresets, RedirectRule, HeaderRule } from '@lib/types';
import Card from '@islands/shared/Card/Card';
import CodeBlock from '@islands/shared/CodeBlock/CodeBlock';
import Toggle from '@islands/shared/Toggle/Toggle';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Textarea from '@islands/shared/Textarea/Textarea';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import styles from './proxy-tab.module.css';

const SUGGESTED_HEADERS = [
  { name: 'X-Frame-Options', value: 'DENY' },
  { name: 'X-Content-Type-Options', value: 'nosniff' },
  { name: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
];

function emptyPresets(): ProxyPresets {
  return { force_https: true, rate_limit: null, basic_auth: null, redirects: [], headers: [] };
}

export default function ProxyTab({ appId }: { appId: string }) {
  const [data, setData] = useState<ProxyConfig | null>(null);
  const [presets, setPresets] = useState<ProxyPresets>(emptyPresets());
  const [loading, setLoading] = useState(true);
  const [savingPresets, setSavingPresets] = useState(false);

  // Advanced mode editing state
  const [advancedDraft, setAdvancedDraft] = useState('');
  const [validateMsg, setValidateMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [showAdvancedConfirm, setShowAdvancedConfirm] = useState(false);
  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [applying, setApplying] = useState(false);

  const load = () => {
    setLoading(true);
    api.getProxyConfig(appId)
      .then(({ data }) => {
        setData(data);
        setPresets({ ...emptyPresets(), ...data.presets });
        setAdvancedDraft(
          data.custom_proxy_config
            ? JSON.stringify(JSON.parse(data.custom_proxy_config), null, 2)
            : JSON.stringify(data.routes ?? {}, null, 2),
        );
      })
      .catch(() => setData(null))
      .finally(() => setLoading(false));
  };

  useEffect(load, [appId]);

  const routesJson = data ? JSON.stringify(data.routes ?? {}, null, 2) : '';
  const advancedMode = data?.has_custom_proxy_config ?? false;

  const copyConfig = async () => {
    try {
      await navigator.clipboard.writeText(routesJson);
      addToast('success', 'Config copied to clipboard');
    } catch {
      addToast('error', 'Could not copy to clipboard');
    }
  };

  const savePresets = async () => {
    setSavingPresets(true);
    try {
      await api.updateProxyPresets(appId, presets);
      addToast('success', 'Presets saved. They apply on the next deploy.');
      load();
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Failed to save presets');
    } finally {
      setSavingPresets(false);
    }
  };

  const validateAdvanced = async () => {
    let parsed: unknown;
    try {
      parsed = JSON.parse(advancedDraft);
    } catch {
      setValidateMsg({ ok: false, text: 'Invalid JSON syntax' });
      return;
    }
    try {
      const { data } = await api.validateProxyConfig(appId, parsed);
      setValidateMsg(
        data.valid
          ? { ok: true, text: 'Config is valid' }
          : { ok: false, text: data.error ?? 'Caddy rejected the config' },
      );
    } catch (e) {
      setValidateMsg({ ok: false, text: e instanceof Error ? e.message : 'Validation failed' });
    }
  };

  const applyAdvanced = async () => {
    setShowAdvancedConfirm(false);
    let parsed: unknown;
    try {
      parsed = JSON.parse(advancedDraft);
    } catch {
      addToast('error', 'Invalid JSON — fix syntax before applying');
      return;
    }
    setApplying(true);
    try {
      await api.setCustomProxyConfig(appId, parsed);
      addToast('success', 'Custom proxy config applied');
      load();
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Failed to apply config');
    } finally {
      setApplying(false);
    }
  };

  const resetConfig = async () => {
    setShowResetConfirm(false);
    try {
      await api.resetProxyConfig(appId);
      addToast('success', 'Reset to auto-generated config');
      load();
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Failed to reset');
    }
  };

  const undoConfig = async () => {
    try {
      const { message } = await api.undoProxyConfig(appId);
      addToast('success', message);
      load();
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Nothing to undo');
    }
  };

  if (loading) return <p class={styles.loading}>Loading proxy config…</p>;

  return (
    <div class={styles.page}>
      {/* Read-only viewer */}
      <Card title="Active routes">
        <div class={styles.viewerHeader}>
          <p class={styles.muted}>
            Auto-generated Caddy configuration for this app's domains.
          </p>
          <Button variant="secondary" size="sm" onClick={copyConfig}>
            <Copy size={14} aria-hidden="true" /> Copy config
          </Button>
        </div>
        <CodeBlock code={routesJson || '// No routes configured'} language="json" />
      </Card>

      {/* Middleware presets */}
      <Card title="Middleware presets">
        {advancedMode ? (
          <p class={styles.presetsDisabled} role="status">
            Presets are disabled while advanced mode is active. The raw config below takes precedence.
          </p>
        ) : (
          <div class={styles.presets}>
            <Toggle
              label="Force HTTPS"
              checked={presets.force_https ?? true}
              onChange={(v) => setPresets({ ...presets, force_https: v })}
              description="Redirect all HTTP traffic to HTTPS (enabled by default via Caddy)."
            />

            <RateLimitSection presets={presets} setPresets={setPresets} />
            <BasicAuthSection presets={presets} setPresets={setPresets} />
            <RedirectSection presets={presets} setPresets={setPresets} />
            <HeaderSection presets={presets} setPresets={setPresets} />

            <div class={styles.actions}>
              <Button onClick={savePresets} loading={savingPresets}>Save presets</Button>
            </div>
          </div>
        )}
      </Card>

      {/* Advanced mode */}
      <Card title="Advanced mode">
        <p class={styles.muted}>
          Edit the raw Caddy JSON config directly. Changes are validated before being applied.
          While advanced mode is active, Icefall will not regenerate this app's proxy config on deploy.
        </p>
        <Textarea
          label="Caddy JSON config"
          name="advanced-config"
          value={advancedDraft}
          rows={14}
          mono
          onChange={setAdvancedDraft}
          helpText="Must be a valid Caddy config object."
        />
        {/* a11y [WCAG 4.1.3]: always-present live region so validation results are announced, not silently mounted */}
        <p
          class={validateMsg ? (validateMsg.ok ? styles.validOk : styles.validErr) : undefined}
          role="status"
          aria-live="polite"
        >
          {validateMsg?.text ?? ''}
        </p>
        <div class={styles.actions}>
          <Button variant="secondary" onClick={validateAdvanced}>Validate</Button>
          <Button onClick={() => setShowAdvancedConfirm(true)} loading={applying}>
            Apply config
          </Button>
          <Button variant="secondary" onClick={undoConfig}>Undo last change</Button>
          {advancedMode && (
            <Button variant="danger" onClick={() => setShowResetConfirm(true)}>
              Reset to default
            </Button>
          )}
        </div>
      </Card>

      <ConfirmDialog
        open={showAdvancedConfirm}
        title="Apply raw proxy config?"
        description="Editing raw proxy config can break routing. The config is validated before applying, and the previous config is saved so you can undo."
        confirmLabel="Apply"
        variant="danger"
        onConfirm={applyAdvanced}
        onCancel={() => setShowAdvancedConfirm(false)}
      />
      <ConfirmDialog
        open={showResetConfirm}
        title="Reset to auto-generated config?"
        description="This discards all custom proxy edits and returns the app to preset-based configuration. It regenerates on the next deploy."
        confirmLabel="Reset"
        variant="danger"
        onConfirm={resetConfig}
        onCancel={() => setShowResetConfirm(false)}
      />
    </div>
  );
}

type SectionProps = { presets: ProxyPresets; setPresets: (p: ProxyPresets) => void };

function RateLimitSection({ presets, setPresets }: SectionProps) {
  const rl = presets.rate_limit;
  const enabled = rl?.enabled ?? false;
  return (
    <div class={styles.preset}>
      <Toggle
        label="Rate limiting"
        checked={enabled}
        onChange={(v) =>
          setPresets({
            ...presets,
            rate_limit: v
              ? { enabled: true, requests: rl?.requests ?? 100, window: rl?.window ?? 'minute', burst: rl?.burst ?? 0, per_ip: rl?.per_ip ?? true }
              : rl ? { ...rl, enabled: false } : null,
          })
        }
        description="Limit how many requests a client can make. Requires the caddy-ratelimit module; otherwise requests are answered with HTTP 429."
      />
      {enabled && rl && (
        <div class={styles.presetBody}>
          <Input
            label="Requests" name="rl-requests" type="number" min={1}
            value={String(rl.requests)}
            onChange={(v) => setPresets({ ...presets, rate_limit: { ...rl, requests: Number(v) || 0 } })}
          />
          <Input
            label="Burst" name="rl-burst" type="number" min={0}
            value={String(rl.burst)}
            onChange={(v) => setPresets({ ...presets, rate_limit: { ...rl, burst: Number(v) || 0 } })}
          />
          <fieldset class={styles.fieldset}>
            <legend class={styles.legend}>Window</legend>
            <label class={styles.radio}>
              <input type="radio" name="rl-window" checked={rl.window === 'minute'}
                onChange={() => setPresets({ ...presets, rate_limit: { ...rl, window: 'minute' } })} />
              Per minute
            </label>
            <label class={styles.radio}>
              <input type="radio" name="rl-window" checked={rl.window === 'second'}
                onChange={() => setPresets({ ...presets, rate_limit: { ...rl, window: 'second' } })} />
              Per second
            </label>
          </fieldset>
          <Toggle
            label="Per IP" checked={rl.per_ip}
            onChange={(v) => setPresets({ ...presets, rate_limit: { ...rl, per_ip: v } })}
            description="Apply the limit per client IP rather than globally."
          />
        </div>
      )}
    </div>
  );
}

function BasicAuthSection({ presets, setPresets }: SectionProps) {
  const ba = presets.basic_auth;
  const enabled = ba?.enabled ?? false;
  const [password, setPassword] = useState('');
  return (
    <div class={styles.preset}>
      <Toggle
        label="HTTP Basic Auth"
        checked={enabled}
        onChange={(v) =>
          setPresets({
            ...presets,
            basic_auth: v
              ? { enabled: true, username: ba?.username ?? '', password_hash: ba?.password_hash ?? '', path: ba?.path ?? null }
              : ba ? { ...ba, enabled: false } : null,
          })
        }
        description="Password-protect the app at the HTTP level. This is not application auth."
      />
      {enabled && ba && (
        <div class={styles.presetBody}>
          <p class={styles.warning}>
            <ShieldAlert size={14} aria-hidden="true" /> This is HTTP-level auth, not application auth.
          </p>
          <Input
            label="Username" name="ba-username" value={ba.username}
            onChange={(v) => setPresets({ ...presets, basic_auth: { ...ba, username: v } })}
          />
          <Input
            label="Password" name="ba-password" type="password" revealable
            value={password}
            helpText="Stored as a bcrypt hash. Leave blank to keep the current password."
            onChange={(v) => { setPassword(v); setPresets({ ...presets, basic_auth: { ...ba, password_hash: v } }); }}
          />
        </div>
      )}
    </div>
  );
}

function RedirectSection({ presets, setPresets }: SectionProps) {
  const setRedirect = (i: number, patch: Partial<RedirectRule>) => {
    const next = presets.redirects.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
    setPresets({ ...presets, redirects: next });
  };
  return (
    <div class={styles.preset}>
      <div class={styles.presetTitleRow}>
        <span class={styles.presetTitle}>Redirect rules</span>
        <Button variant="secondary" size="sm"
          onClick={() => setPresets({ ...presets, redirects: [...presets.redirects, { from: '', to: '', status: 301 }] })}>
          <Plus size={14} aria-hidden="true" /> Add redirect
        </Button>
      </div>
      {presets.redirects.map((r, i) => (
        <div class={styles.ruleRow} key={i}>
          <Input label="From path" name={`redir-from-${i}`} value={r.from}
            onChange={(v) => setRedirect(i, { from: v })} />
          <Input label="To URL" name={`redir-to-${i}`} type="url" value={r.to}
            onChange={(v) => setRedirect(i, { to: v })} />
          <fieldset class={styles.fieldset}>
            <legend class={styles.legend}>Status</legend>
            <label class={styles.radio}>
              <input type="radio" name={`redir-status-${i}`} checked={r.status === 301}
                onChange={() => setRedirect(i, { status: 301 })} /> 301
            </label>
            <label class={styles.radio}>
              <input type="radio" name={`redir-status-${i}`} checked={r.status === 302}
                onChange={() => setRedirect(i, { status: 302 })} /> 302
            </label>
          </fieldset>
          <Button variant="ghost" size="sm" aria-label={`Remove redirect rule ${i + 1}`}
            onClick={() => setPresets({ ...presets, redirects: presets.redirects.filter((_, idx) => idx !== i) })}>
            <Trash2 size={14} aria-hidden="true" />
          </Button>
        </div>
      ))}
    </div>
  );
}

function HeaderSection({ presets, setPresets }: SectionProps) {
  const setHeader = (i: number, patch: Partial<HeaderRule>) => {
    const next = presets.headers.map((h, idx) => (idx === i ? { ...h, ...patch } : h));
    setPresets({ ...presets, headers: next });
  };
  const addSuggested = (name: string, value: string) => {
    if (presets.headers.some((h) => h.name === name)) return;
    setPresets({ ...presets, headers: [...presets.headers, { name, value }] });
  };
  return (
    <div class={styles.preset}>
      <div class={styles.presetTitleRow}>
        <span class={styles.presetTitle}>Custom response headers</span>
        <Button variant="secondary" size="sm"
          onClick={() => setPresets({ ...presets, headers: [...presets.headers, { name: '', value: '' }] })}>
          <Plus size={14} aria-hidden="true" /> Add header
        </Button>
      </div>
      <div class={styles.suggestions}>
        <span class={styles.muted}>Suggested:</span>
        {SUGGESTED_HEADERS.map((h) => (
          <button type="button" class={styles.chip} key={h.name} onClick={() => addSuggested(h.name, h.value)}>
            {h.name}
          </button>
        ))}
      </div>
      {presets.headers.map((h, i) => (
        <div class={styles.ruleRow} key={i}>
          <Input label="Name" name={`hdr-name-${i}`} value={h.name}
            onChange={(v) => setHeader(i, { name: v })} />
          <Input label="Value" name={`hdr-value-${i}`} value={h.value}
            onChange={(v) => setHeader(i, { value: v })} />
          <Button variant="ghost" size="sm" aria-label={`Remove header ${i + 1}`}
            onClick={() => setPresets({ ...presets, headers: presets.headers.filter((_, idx) => idx !== i) })}>
            <Trash2 size={14} aria-hidden="true" />
          </Button>
        </div>
      ))}
    </div>
  );
}
