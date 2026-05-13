import { useState } from 'preact/hooks';
import type { ApiToken } from '@lib/types';
import { formatRelativeTime } from '@lib/format';
import Button from '@islands/shared/Button/Button';
import { Key, Trash2, Copy } from 'lucide-preact';
import styles from '../users-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  tokens: ApiToken[];
  newTokenValue: string;
  onCreateToken: (name: string) => Promise<void>;
  onRevokeToken: (tokenId: string) => void;
  onDismissNewToken: () => void;
};

export default function ApiTokensSection({
  tokens,
  newTokenValue,
  onCreateToken,
  onRevokeToken,
  onDismissNewToken,
}: Props) {
  const [showCreateToken, setShowCreateToken] = useState(false);
  const [tokenName, setTokenName] = useState('');
  const [submitting, setSubmitting] = useState(false);

  async function handleCreate() {
    if (!tokenName.trim()) return;
    setSubmitting(true);
    await onCreateToken(tokenName.trim());
    setTokenName('');
    setShowCreateToken(false);
    setSubmitting(false);
  }

  return (
    <section class={styles.section}>
      <div class={styles.sectionHeader}>
        <h2 class={styles.sectionTitle}>API Tokens</h2>
        <Button variant="primary" onClick={() => setShowCreateToken(true)}>
          <Key size={14} aria-hidden="true" /> Create Token
        </Button>
      </div>

      {newTokenValue && (
        <div class={styles.tokenBanner} role="alert">
          <p class={styles.tokenBannerLabel}>Copy your new token -- it won't be shown again:</p>
          <div class={styles.tokenRow}>
            <code class={styles.tokenValue}>{newTokenValue}</code>
            <button type="button" class={styles.iconButton} onClick={() => navigator.clipboard.writeText(newTokenValue)} aria-label="Copy token">
              <Copy size={14} aria-hidden="true" />
            </button>
          </div>
          <Button variant="ghost" onClick={onDismissNewToken}>Dismiss</Button>
        </div>
      )}

      {showCreateToken && (
        <div class={styles.card}>
          <label htmlFor="token-name" class={formStyles.label}>Token Name</label>
          <input id="token-name" class={formStyles.input} value={tokenName} onInput={e => setTokenName((e.target as HTMLInputElement).value)} placeholder="CI/CD pipeline" />
          <div class={styles.cardActions}>
            <Button variant="ghost" onClick={() => setShowCreateToken(false)}>Cancel</Button>
            <Button variant="primary" onClick={handleCreate} loading={submitting} disabled={!tokenName.trim()}>Create</Button>
          </div>
        </div>
      )}

      <div class={styles.tableCard}>
        <table class={styles.table}>
          <thead>
            <tr class={styles.tableRow}>
              <th class={styles.th}>Name</th>
              <th class={styles.th}>Prefix</th>
              <th class={styles.th}>Last Used</th>
              <th class={styles.th}>Expires</th>
              <th class={styles.th}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {tokens.map(t => (
              <tr key={t.id} class={styles.tableRow}>
                <td class={styles.td}>{t.name}</td>
                <td class={styles.tdMono}>{t.prefix}...</td>
                <td class={styles.tdMuted}>{t.last_used_at ? formatRelativeTime(t.last_used_at) : 'Never'}</td>
                <td class={styles.tdMuted}>{t.expires_at ? new Date(t.expires_at).toLocaleDateString() : 'Never'}</td>
                <td class={styles.td}>
                  <button type="button" onClick={() => onRevokeToken(t.id)} class={styles.iconButton} aria-label={`Revoke ${t.name}`}>
                    <Trash2 size={14} aria-hidden="true" />
                  </button>
                </td>
              </tr>
            ))}
            {tokens.length === 0 && (
              <tr>
                <td class={styles.emptyRow} colSpan={5}>No API tokens yet.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
