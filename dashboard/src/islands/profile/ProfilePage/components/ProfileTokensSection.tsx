import { useState } from 'preact/hooks';
import type { ApiToken } from '@lib/types';
import { formatRelativeTime } from '@lib/format';
import { Key, Copy, Plus, Trash2 } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import styles from '../profile-page.module.css';

type Props = {
  tokens: ApiToken[];
  onCreateToken: (name: string) => Promise<string>;
  onRevokeToken: (tokenId: string) => Promise<void>;
};

export default function ProfileTokensSection({ tokens, onCreateToken, onRevokeToken }: Props) {
  const [showCreateToken, setShowCreateToken] = useState(false);
  const [tokenName, setTokenName] = useState('');
  const [newTokenValue, setNewTokenValue] = useState('');
  const [tokenSubmitting, setTokenSubmitting] = useState(false);

  async function handleCreateToken() {
    if (!tokenName.trim()) return;
    setTokenSubmitting(true);
    try {
      const token = await onCreateToken(tokenName.trim());
      setNewTokenValue(token);
      setTokenName('');
      setShowCreateToken(false);
    } catch {}
    setTokenSubmitting(false);
  }

  return (
    <section class={styles.section} aria-labelledby="tokens-heading">
      <div class={styles.sectionHeader}>
        <h2 id="tokens-heading" class={styles.sectionHeading}>
          <Key size={18} aria-hidden="true" /> API Tokens
        </h2>
        <Button variant="secondary" size="sm" onClick={() => setShowCreateToken(true)}>
          <Plus size={14} aria-hidden="true" /> Create Token
        </Button>
      </div>

      {newTokenValue && (
        <div class={styles.tokenBanner} role="alert">
          <p class={styles.tokenBannerLabel}>Copy your new token. It won't be shown again:</p>
          <div class={styles.tokenRow}>
            <code class={styles.tokenValue}>{newTokenValue}</code>
            <button
              type="button"
              class={styles.iconButton}
              onClick={() => navigator.clipboard.writeText(newTokenValue)}
              aria-label="Copy token to clipboard"
            >
              <Copy size={14} aria-hidden="true" />
            </button>
          </div>
          <Button variant="ghost" size="sm" onClick={() => setNewTokenValue('')}>Dismiss</Button>
        </div>
      )}

      {showCreateToken && (
        <div class={styles.card}>
          {/* a11y [1.3.1]: label associated with input */}
          <Input
            label="Token Name"
            name="token-name"
            id="token-name"
            value={tokenName}
            onChange={setTokenName}
            placeholder="CI/CD pipeline"
          />
          <div class={styles.cardActions}>
            <Button variant="ghost" onClick={() => { setShowCreateToken(false); setTokenName(''); }}>Cancel</Button>
            <Button variant="primary" onClick={handleCreateToken} loading={tokenSubmitting} disabled={!tokenName.trim()}>Create</Button>
          </div>
        </div>
      )}

      <div class={styles.tableCard}>
        <table class={styles.table}>
          <thead>
            <tr class={styles.tableRow}>
              <th class={styles.th}>Name</th>
              <th class={styles.th}>Last Used</th>
              <th class={styles.th}>Expires</th>
              <th class={styles.th}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {tokens.map(t => (
              <tr key={t.id} class={styles.tableRow}>
                <td class={styles.td}>{t.name}</td>
                <td class={styles.tdMuted}>{t.last_used_at ? formatRelativeTime(t.last_used_at) : 'Never'}</td>
                <td class={styles.tdMuted}>{t.expires_at ? new Date(t.expires_at).toLocaleDateString() : 'Never'}</td>
                <td class={styles.td}>
                  <button
                    type="button"
                    onClick={() => onRevokeToken(t.id)}
                    class={styles.iconButton}
                    aria-label={`Revoke token ${t.name}`}
                  >
                    <Trash2 size={14} aria-hidden="true" />
                  </button>
                </td>
              </tr>
            ))}
            {tokens.length === 0 && (
              <tr>
                <td class={styles.emptyRow} colSpan={4}>No API tokens yet.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
