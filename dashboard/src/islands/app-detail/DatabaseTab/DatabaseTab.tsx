import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import Button from '@islands/shared/Button/Button';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import DatabaseBrowser from '@islands/databases/DatabaseBrowser/DatabaseBrowser';
import { formatRelativeTime } from '@lib/format';
import { Database, Plus, Link2, Unlink, Eye, EyeOff, Copy, Trash2 } from 'lucide-preact';
import styles from './database-tab.module.css';
import formStyles from '@styles/form.module.css';

type ManagedDb = {
  id: string;
  name: string;
  db_type: string;
  container_id: string | null;
  credentials: string;
  app_id: string | null;
  created_at: string;
}

type Props = {
  appId: string;
}

const DB_ICONS: Record<string, string> = {
  postgres: 'PG', mysql: 'MY', redis: 'RD', mongo: 'MG',
};

const DB_COLORS: Record<string, string> = {
  postgres: 'oklch(0.55 0.15 250)', mysql: 'oklch(0.55 0.15 40)',
  redis: 'oklch(0.55 0.20 25)', mongo: 'oklch(0.55 0.15 140)',
};

export default function DatabaseTab({ appId }: Props) {
  const [linked, setLinked] = useState<ManagedDb[]>([]);
  const [unlinked, setUnlinked] = useState<ManagedDb[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedDb, setSelectedDb] = useState<ManagedDb | null>(null);
  const [showLink, setShowLink] = useState(false);
  const [showCredentials, setShowCredentials] = useState(false);
  const [linking, setLinking] = useState(false);

  async function loadDatabases() {
    try {
      const { data } = await api.listDatabases();
      setLinked(data.filter((db: ManagedDb) => db.app_id === appId));
      setUnlinked(data.filter((db: ManagedDb) => !db.app_id));
    } catch {}
    setLoading(false);
  }

  useEffect(() => { loadDatabases(); }, [appId]);

  async function handleLink(dbId: string) {
    setLinking(true);
    try {
      await api.linkDatabase(dbId, appId);
      await loadDatabases();
      setShowLink(false);
    } catch {}
    setLinking(false);
  }

  async function handleUnlink(db: ManagedDb) {
    try {
      await api.unlinkDatabase(db.id, appId);
      await loadDatabases();
      if (selectedDb?.id === db.id) setSelectedDb(null);
    } catch {}
  }

  function getConnectionString(db: ManagedDb): string {
    try {
      return JSON.parse(db.credentials).connection_string || '';
    } catch { return ''; }
  }

  if (loading) return <p class={styles.loadingText}>Loading databases...</p>;

  if (selectedDb) {
    const connStr = getConnectionString(selectedDb);
    return (
      <div>
        <button type="button" onClick={() => setSelectedDb(null)} class={styles.backButton}>
          ← Back to databases
        </button>

        <div class={styles.detailHeader}>
          <span class={styles.dbIcon} style={{ background: DB_COLORS[selectedDb.db_type] || 'var(--color-primary)' }}>
            {DB_ICONS[selectedDb.db_type] || 'DB'}
          </span>
          <div>
            <h3 class={styles.detailTitle}>{selectedDb.name}</h3>
            <span class={styles.detailSubtitle}>{selectedDb.db_type}</span>
          </div>
        </div>

        <div class={styles.card}>
          <h4 class={styles.cardTitle}>Connection</h4>
          <div class={styles.connectionRow}>
            <code class={showCredentials ? styles.connRevealed : styles.connHidden}>
              {showCredentials ? (connStr || 'No credentials stored') : '••••••••••••••••••••••••'}
            </code>
            <button type="button" onClick={() => setShowCredentials(!showCredentials)} class={styles.iconButton} aria-label={showCredentials ? 'Hide' : 'Show'}>
              {showCredentials ? <EyeOff size={16} aria-hidden="true" /> : <Eye size={16} aria-hidden="true" />}
            </button>
            <button type="button" onClick={() => navigator.clipboard.writeText(connStr)} class={styles.iconButton} aria-label="Copy">
              <Copy size={16} aria-hidden="true" />
            </button>
          </div>
        </div>

        <div class={styles.card}>
          <DatabaseBrowser dbId={selectedDb.id} dbType={selectedDb.db_type} />
        </div>
      </div>
    );
  }

  return (
    <div>
      <div class={styles.header}>
        <p class={styles.description}>Databases linked to this application.</p>
        <Button variant="secondary" onClick={() => setShowLink(!showLink)}>
          <Plus size={14} aria-hidden="true" /> Link Database
        </Button>
      </div>

      {showLink && (
        <div class={styles.linkCard}>
          <h4 class={styles.linkTitle}>Link an existing database</h4>
          {unlinked.length === 0 ? (
            <p class={styles.emptyHint}>No unlinked databases available. Create one on the <a href="/databases" data-astro-prefetch="hover">Databases page</a> first.</p>
          ) : (
            <div class={styles.linkGrid}>
              {unlinked.map(db => (
                <div key={db.id} class={styles.linkItem}>
                  <span class={styles.dbIconSmall} style={{ background: DB_COLORS[db.db_type] || 'var(--color-primary)' }}>
                    {DB_ICONS[db.db_type] || 'DB'}
                  </span>
                  <div class={styles.linkItemInfo}>
                    <span class={styles.linkItemName}>{db.name}</span>
                    <span class={styles.linkItemType}>{db.db_type}</span>
                  </div>
                  <Button variant="primary" size="sm" onClick={() => handleLink(db.id)} loading={linking}>
                    <Link2 size={12} aria-hidden="true" /> Link
                  </Button>
                </div>
              ))}
            </div>
          )}
          <div class={styles.linkActions}>
            <Button variant="ghost" onClick={() => setShowLink(false)}>Cancel</Button>
          </div>
        </div>
      )}

      {linked.length === 0 && !showLink ? (
        <div class={styles.emptyState}>
          <Database size={40} class={styles.emptyIcon} aria-hidden="true" />
          <p class={styles.emptyTitle}>No databases linked</p>
          <p class={styles.emptyHint}>Link a database to inject its connection string as an <a href={`/apps/${appId}/env`} data-astro-prefetch="hover">environment variable</a>.</p>
        </div>
      ) : (
        <div class={styles.dbGrid}>
          {linked.map(db => (
            <div key={db.id} class={styles.dbCard}>
              <button type="button" class={styles.dbCardMain} onClick={() => setSelectedDb(db)}>
                <span class={styles.dbIconSmall} style={{ background: DB_COLORS[db.db_type] || 'var(--color-primary)' }}>
                  {DB_ICONS[db.db_type] || 'DB'}
                </span>
                <div>
                  <div class={styles.dbCardName}>{db.name}</div>
                  <div class={styles.dbCardType}>{db.db_type}</div>
                </div>
                <div class={styles.dbCardStatus}>
                  <StatusDot status={db.container_id ? 'online' : 'stopped'} showLabel={false} />
                </div>
              </button>
              <div class={styles.dbCardActions}>
                <button type="button" class={styles.iconButton} onClick={() => handleUnlink(db)} aria-label={`Unlink ${db.name}`}>
                  <Unlink size={14} aria-hidden="true" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
