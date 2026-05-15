import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import type { Domain } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import { Plus, Trash2, Shield, Globe } from 'lucide-preact';
import styles from './domains-tab.module.css';

type Props = {
  appId: string;
}

export default function DomainsTab({ appId }: Props) {
  const [domains, setDomains] = useState<Domain[]>([]);
  const [loading, setLoading] = useState(true);
  const [newDomain, setNewDomain] = useState('');
  const [newPath, setNewPath] = useState('');
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    api.listDomains(appId).then(({ data }) => { setDomains(data); setLoading(false); }).catch(() => setLoading(false));
  }, [appId]);

  async function handleAdd() {
    if (!newDomain.trim()) return;
    const pathValue = newPath.trim() || undefined;
    if (pathValue && !pathValue.startsWith('/')) return;
    setAdding(true);
    try {
      const { data } = await api.addDomain(appId, newDomain.trim(), pathValue);
      setDomains((prev) => [...prev, data]);
      setNewDomain('');
      setNewPath('');
    } catch { /* show error */ }
    setAdding(false);
  }

  if (loading) return <p class={styles.loadingText}>Loading domains...</p>;

  return (
    <div>
      <div class={styles.headerActions}>
        <div class={styles.inputGroup}>
          <Input
            label="New domain"
            name="new-domain"
            id="new-domain-input"
            value={newDomain}
            onChange={setNewDomain}
            placeholder="example.com"
          />
          <Input
            label="Path prefix (optional)"
            name="new-path"
            id="new-path-input"
            value={newPath}
            onChange={setNewPath}
            placeholder="/api (optional)"
            className={styles.pathInput}
          />
          <Button variant="primary" onClick={handleAdd} loading={adding}>
            <Plus size={14} /> Add Domain
          </Button>
        </div>
      </div>

      {domains.length === 0 ? (
        <div class={styles.emptyState}>
          <Globe size={32} class={styles.emptyIcon} />
          <p>No custom domains configured.</p>
          <p class={styles.emptyHint}>
            Add a domain and point its DNS to your server.
          </p>
        </div>
      ) : (
        <div class={styles.tableCard}>
          <table class={styles.table}>
            <thead>
              <tr class={styles.tableRow}>
                {['Domain', 'Path', 'SSL', 'DNS Status', 'Actions'].map((h) => (
                  <th key={h} class={styles.th}>
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {domains.map((d) => (
                <tr key={d.id} class={styles.tableRow}>
                  <td class={styles.domainCell}>
                    <div class={styles.domainName}>
                      <Shield size={14} class={d.ssl_status === 'active' ? styles.sslActive : styles.sslInactive} />
                      {d.domain}
                    </div>
                  </td>
                  <td class={styles.td}>
                    {d.path ? (
                      <code class={styles.pathBadge}>{d.path}</code>
                    ) : (
                      <span class={styles.allPaths}>/*</span>
                    )}
                  </td>
                  <td class={`${styles.td} ${d.ssl_status === 'active' ? styles.sslActive : styles.sslInactive}`}>
                    {d.ssl_status === 'active' ? 'Valid' : d.ssl_status}
                  </td>
                  <td class={styles.td}>
                    <span class={d.verified ? styles.dnsConfigured : styles.dnsPending}>
                      <span class={styles.dnsDot} aria-hidden="true" />
                      {d.verified ? 'Configured' : 'Pending'}
                    </span>
                  </td>
                  <td class={styles.td}>
                    {/* a11y [4.1.2]: aria-label on icon-only button */}
                    <button class={styles.iconButton} aria-label="Delete domain">
                      <Trash2 size={14} />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
