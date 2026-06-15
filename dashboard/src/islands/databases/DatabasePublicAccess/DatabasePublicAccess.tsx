import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import type { PublicAccess } from '@lib/types';
import Toggle from '@islands/shared/Toggle/Toggle';
import Alert from '@islands/shared/Alert/Alert';
import Input from '@islands/shared/Input/Input';
import { Copy, Check, Globe } from 'lucide-preact';
import styles from './database-public-access.module.css';

type Props = {
  dbId: string;
};

// IF-172: enable/disable raw TCP access to a managed database and show the
// external connection details. The toggle is the source of truth; the IP
// whitelist field is editable only while enabling.
export default function DatabasePublicAccess({ dbId }: Props) {
  const [access, setAccess] = useState<PublicAccess | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [ipWhitelist, setIpWhitelist] = useState('');
  const [error, setError] = useState('');
  const [status, setStatus] = useState('');
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    api.getDatabasePublicAccess(dbId)
      .then(({ data }) => {
        if (!active) return;
        setAccess(data);
        if (data.ip_whitelist) setIpWhitelist(data.ip_whitelist);
      })
      .catch(() => active && setError('Could not load public access status'))
      .finally(() => active && setLoading(false));
    return () => { active = false; };
  }, [dbId]);

  async function handleToggle(next: boolean) {
    setBusy(true);
    setError('');
    setStatus('');
    try {
      if (next) {
        const { data } = await api.enableDatabasePublicAccess(dbId, ipWhitelist.trim() || undefined);
        setAccess(data);
        setStatus(data.port ? `Public access enabled on port ${data.port}.` : 'Public access enabled.');
      } else {
        const { data } = await api.disableDatabasePublicAccess(dbId);
        setAccess(data);
        setStatus('Public access disabled.');
      }
    } catch (e) {
      // The backend returns a clear message for the common failure (no L4 module).
      setError(e instanceof Error ? e.message : 'Failed to update public access');
    } finally {
      setBusy(false);
    }
  }

  async function copy(value: string, key: string) {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(key);
      setTimeout(() => setCopied(null), 1500);
    } catch { /* clipboard unavailable — no-op */ }
  }

  if (loading) {
    return <p class={styles.loading}>Loading public access…</p>;
  }

  const enabled = access?.enabled ?? false;
  const conn = access?.connection ?? null;
  const isOpenToInternet = enabled && !access?.ip_whitelist;

  return (
    <div class={styles.wrap}>
      <div class={styles.header}>
        <Globe size={16} aria-hidden="true" class={styles.headerIcon} />
        <h3 class={styles.title}>Public Access</h3>
      </div>
      <p class={styles.intro}>
        Expose this database on a public TCP port so external tools (pgAdmin, TablePlus,
        DBeaver) can connect directly.
      </p>

      <Toggle
        label="Allow public access"
        description="Routes a public port through Caddy to this database."
        checked={enabled}
        disabled={busy}
        onChange={handleToggle}
      />

      {/* a11y [WCAG 4.1.3]: announce enable/disable result to assistive tech — the
          connection panel appearing/disappearing is otherwise silent. */}
      <p role="status" aria-live="polite" class={styles.status}>{status}</p>

      {/* IP whitelist is only meaningful while configuring an enable. */}
      {!enabled && (
        <div class={styles.field}>
          <Input
            label="IP whitelist (optional)"
            name="public-access-ip-whitelist"
            id="public-access-ip-whitelist"
            value={ipWhitelist}
            onChange={setIpWhitelist}
            placeholder="1.2.3.4, 10.0.0.0/8"
            helpText="Comma-separated IPs or CIDRs. Leave blank to allow any source (not recommended)."
            disabled={busy}
          />
        </div>
      )}

      {error && <Alert variant="error">{error}</Alert>}

      {isOpenToInternet && (
        <Alert variant="warning">
          This exposes the database to the internet. Use strong credentials and consider
          IP whitelisting.
        </Alert>
      )}

      {enabled && conn && (
        <div class={styles.connection}>
          {access?.ip_whitelist && (
            <p class={styles.whitelistNote}>
              Allowed sources: <span class={styles.mono}>{access.ip_whitelist}</span>
            </p>
          )}
          <dl class={styles.connGrid}>
            <ConnRow label="Host" value={conn.host} k="host" copied={copied} onCopy={copy} />
            <ConnRow label="Port" value={String(conn.port)} k="port" copied={copied} onCopy={copy} />
            <ConnRow label="User" value={conn.user} k="user" copied={copied} onCopy={copy} />
            <ConnRow label="Connection URL" value={conn.url} k="url" copied={copied} onCopy={copy} mono />
          </dl>
        </div>
      )}
    </div>
  );
}

function ConnRow({
  label, value, k, copied, onCopy, mono,
}: {
  label: string;
  value: string;
  k: string;
  copied: string | null;
  onCopy: (value: string, key: string) => void;
  mono?: boolean;
}) {
  return (
    <div class={styles.connRow}>
      <dt class={styles.connLabel}>{label}</dt>
      <dd class={styles.connValue}>
        <code class={mono ? styles.connCodeWrap : styles.connCode}>{value}</code>
        {/* a11y [4.1.2]: aria-label on icon-only copy button */}
        <button
          type="button"
          class={styles.copyButton}
          onClick={() => onCopy(value, k)}
          aria-label={`Copy ${label.toLowerCase()}`}
        >
          {copied === k ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
        </button>
      </dd>
    </div>
  );
}
