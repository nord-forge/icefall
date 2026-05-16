import { useEffect, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $databases, $databasesLoaded } from '@stores/databases';
import type { ManagedDb } from '@stores/databases';
import Button from '@islands/shared/Button/Button';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import Select from '@islands/shared/Select/Select';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import DatabaseBrowser from '@islands/databases/DatabaseBrowser/DatabaseBrowser';
import { formatRelativeTime, formatBytes } from '@lib/format';
import { Plus, Database, Trash2, Copy, Eye, EyeOff, RefreshCw, Download, RotateCcw } from 'lucide-preact';
import { SkeletonCard } from '@islands/shared/Skeleton/Skeleton';
import Input from '@islands/shared/Input/Input';
import styles from './databases-page.module.css';
import formStyles from '@styles/form.module.css';

type _ManagedDb = {
  id: string;
  name: string;
  db_type: string;
  container_id: string | null;
  credentials: string;
  backup_schedule: string | null;
  app_id: string | null;
  created_at: string;
}

type BackupInfo = {
  id: string;
  filename: string;
  size_bytes: number;
  created_at: string;
  status: string;
}

const DB_ICONS: Record<string, string> = {
  postgres: 'PG',
  mysql: 'MY',
  redis: 'RD',
  mongo: 'MG',
};

const DB_COLORS: Record<string, string> = {
  postgres: 'oklch(0.55 0.15 250)',
  mysql: 'oklch(0.55 0.15 40)',
  redis: 'oklch(0.55 0.20 25)',
  mongo: 'oklch(0.55 0.15 140)',
};

export default function DatabasesPage() {
  const cachedDbs = useStore($databases);
  const wasLoaded = useStore($databasesLoaded);
  const [dbs, setDbs] = useState<ManagedDb[]>(cachedDbs);
  const [loading, setLoading] = useState(!wasLoaded);
  const [selectedDb, setSelectedDb] = useState<ManagedDb | null>(null);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [showCredentials, setShowCredentials] = useState(false);
  const [creating, setCreating] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [saveMessage, setSaveMessage] = useState('');
  const [newDb, setNewDb] = useState({ name: '', db_type: 'postgres', memory_mb: '' });

  useEffect(() => {
    fetch('/api/v1/databases', { credentials: 'same-origin' }).then(r => r.json()).then(d => {
      const all = d.data || [];
      const unlinked = all.filter((db: ManagedDb) => !db.app_id);
      setDbs(unlinked);
      $databases.set(unlinked);
      $databasesLoaded.set(true);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (selectedDb) {
      fetch(`/api/v1/databases/${selectedDb.id}/backups`).then(r => r.json()).then(d => setBackups(d.data || [])).catch(() => {});
    }
  }, [selectedDb?.id]);

  async function handleCreate() {
    setCreating(true);
    try {
      const res = await fetch('/api/v1/databases', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newDb.name,
          db_type: newDb.db_type,
          memory_mb: newDb.memory_mb ? parseInt(newDb.memory_mb) : undefined,
        }),
      });
      if (res.ok) {
        const { data } = await res.json();
        setDbs(prev => [...prev, data]);
        setShowCreate(false);
        setNewDb({ name: '', db_type: 'postgres', memory_mb: '' });
      }
    } catch {}
    setCreating(false);
  }

  async function handleDelete(id: string) {
    await fetch(`/api/v1/databases/${id}`, { method: 'DELETE' });
    setDbs(prev => prev.filter(d => d.id !== id));
    if (selectedDb?.id === id) setSelectedDb(null);
  }

  async function handleBackup(id: string) {
    await fetch(`/api/v1/databases/${id}/backup`, { method: 'POST' });
    const res = await fetch(`/api/v1/databases/${id}/backups`);
    const d = await res.json();
    setBackups(d.data || []);
  }

  function getConnectionString(db: ManagedDb): string {
    try {
      const creds = JSON.parse(db.credentials);
      return creds.connection_string || '';
    } catch { return ''; }
  }

  if (loading && dbs.length === 0) return (
    <div>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Databases</h1>
        <Button variant="primary" disabled>
          <Plus size={14} aria-hidden="true" /> Add Database
        </Button>
      </div>
      <div class={styles.dbGrid}>
        <SkeletonCard />
        <SkeletonCard />
        <SkeletonCard />
      </div>
    </div>
  );

  if (selectedDb) {
    const connStr = getConnectionString(selectedDb);
    return (
      <div>
        <button onClick={() => setSelectedDb(null)} class={styles.backButton}>
          ← Back to databases
        </button>

        <div class={styles.detailHeader}>
          <span class={styles.dbIcon} style={{ background: DB_COLORS[selectedDb.db_type] || 'var(--color-primary)' }}>
            {DB_ICONS[selectedDb.db_type] || 'DB'}
          </span>
          <div>
            <h1 class={styles.detailTitle}>{selectedDb.name}</h1>
            <span class={styles.detailSubtitle}>{selectedDb.db_type}</span>
          </div>
        </div>

        {saveMessage && <p role="status" aria-live="polite" class={styles.saveMessage}>{saveMessage}</p>}
        <div class={styles.detailGrid}>
          <div class={styles.card}>
            <h3 class={styles.cardTitle}>Connection</h3>
            <div class={styles.connectionRow}>
              <code class={showCredentials ? styles.connectionStringRevealed : styles.connectionStringHidden}>
                {showCredentials ? (connStr || 'No credentials stored. Recreate the database to generate new credentials') : '••••••••••••••••••••••••'}
              </code>
              {/* a11y [4.1.2]: aria-label on icon-only button */}
              <button onClick={() => setShowCredentials(!showCredentials)} class={styles.iconButton} aria-label={showCredentials ? 'Hide credentials' : 'Show credentials'}>
                {showCredentials ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>
              {/* a11y [4.1.2]: aria-label on icon-only button */}
              <button onClick={() => navigator.clipboard.writeText(connStr)} class={styles.iconButton} aria-label="Copy connection string">
                <Copy size={16} />
              </button>
            </div>
          </div>

          <div class={styles.card}>
            <div class={styles.backupHeader}>
              <h3 class={styles.cardTitle}>Backups</h3>
              <Button variant="secondary" size="sm" onClick={() => handleBackup(selectedDb.id)}>
                <RefreshCw size={12} /> Backup Now
              </Button>
            </div>
            {backups.length === 0 ? (
              <p class={styles.noBackups}>No backups yet.</p>
            ) : (
              <table class={styles.table}>
                <thead>
                  <tr class={styles.tableRow}>
                    {['Filename', 'Size', 'Created', 'Status', 'Actions'].map(h => (
                      <th key={h} class={styles.th}>{h}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {backups.map(b => (
                    <tr key={b.id} class={styles.tableRow}>
                      <td class={styles.tdMono}>{b.filename}</td>
                      <td class={styles.tdSecondary}>{formatBytes(b.size_bytes)}</td>
                      <td class={styles.tdMuted}>{b.created_at ? formatRelativeTime(b.created_at) : '-'}</td>
                      <td class={styles.td}><StatusDot status={b.status === 'completed' ? 'success' : 'failed'} /></td>
                      <td class={styles.tdActions}>
                        {b.status === 'completed' && (
                          <>
                            <button
                              type="button"
                              class={styles.iconButton}
                              onClick={() => window.open(`/api/v1/databases/${selectedDb.id}/backups/${b.id}/download`, '_blank')}
                              aria-label={`Download ${b.filename}`}
                            >
                              <Download size={14} aria-hidden="true" />
                            </button>
                            <button
                              type="button"
                              class={styles.iconButton}
                              onClick={() => {
                                if (confirm(`Restore from ${b.filename}? This will overwrite the current database.`)) {
                                  fetch(`/api/v1/databases/${selectedDb.id}/backups/${b.id}/restore`, { method: 'POST' })
                                    .then(() => setSaveMessage('Restore started'))
                                    .catch(() => setSaveMessage('Restore failed'));
                                }
                              }}
                              aria-label={`Restore from ${b.filename}`}
                            >
                              <RotateCcw size={14} aria-hidden="true" />
                            </button>
                          </>
                        )}
                        <button
                          type="button"
                          class={styles.iconButton}
                          onClick={() => {
                            fetch(`/api/v1/databases/${selectedDb.id}/backups/${b.id}`, { method: 'DELETE' })
                              .then(() => setBackups(prev => prev.filter(x => x.id !== b.id)))
                              .catch(() => {});
                          }}
                          aria-label={`Delete ${b.filename}`}
                        >
                          <Trash2 size={14} aria-hidden="true" />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div class={styles.card}>
            <DatabaseBrowser dbId={selectedDb.id} dbType={selectedDb.db_type} />
          </div>

          <div class={styles.dangerCard}>
            <div class={styles.dangerRow}>
              <div>
                <p class={styles.dangerLabel}>Delete Database</p>
                <p class={styles.dangerDescription}>This will permanently delete the database and all its data.</p>
              </div>
              <Button variant="danger" onClick={() => setConfirmDelete(true)}>
                <Trash2 size={14} /> Delete
              </Button>
            </div>
          </div>

          <ConfirmDialog
            open={confirmDelete}
            title="Delete database?"
            description={`This will permanently delete "${selectedDb.name}" and all its data, including backups. This action cannot be undone.`}
            confirmLabel="Delete"
            variant="danger"
            loading={deleting}
            onConfirm={async () => {
              setDeleting(true);
              try {
                await handleDelete(selectedDb.id);
              } finally {
                setDeleting(false);
                setConfirmDelete(false);
              }
            }}
            onCancel={() => setConfirmDelete(false)}
          />
        </div>
      </div>
    );
  }

  return (
    <div>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Databases</h1>
        <Button variant="primary" onClick={() => setShowCreate(true)}>
          <Plus size={14} /> Add Database
        </Button>
      </div>

      {showCreate && (
        <div class={styles.createCard}>
          <h3 class={styles.createTitle}>Create Database</h3>
          <div class={formStyles.fieldRow}>
            <Input
              label="Name"
              name="db-create-name"
              id="db-create-name"
              value={newDb.name}
              onChange={(v) => setNewDb({ ...newDb, name: v })}
              placeholder="my-database"
            />
            <div>
              <label htmlFor="db-create-type" class={formStyles.label}>Type</label>
              <Select
                id="db-create-type"
                options={[
                  { value: 'postgres', label: 'PostgreSQL' },
                  { value: 'mysql', label: 'MySQL' },
                  { value: 'redis', label: 'Redis' },
                  { value: 'mongo', label: 'MongoDB' },
                ]}
                value={newDb.db_type}
                onChange={(v) => setNewDb({ ...newDb, db_type: v })}
                fullWidth
              />
            </div>
          </div>
          <div class={styles.createActions}>
            <Button variant="ghost" onClick={() => setShowCreate(false)}>Cancel</Button>
            <Button variant="primary" onClick={handleCreate} loading={creating} disabled={!newDb.name.trim()}>
              <Database size={14} /> Create
            </Button>
          </div>
        </div>
      )}

      {dbs.length === 0 && !showCreate ? (
        <div class={styles.emptyState}>
          <Database size={40} class={styles.emptyIcon} />
          <p class={styles.emptyTitle}>No databases yet</p>
          <p class={styles.emptyHint}>Provision a database with one click.</p>
        </div>
      ) : (
        <div class={styles.dbGrid}>
          {dbs.map(db => (
            <button
              key={db.id}
              type="button"
              onClick={() => setSelectedDb(db)}
              class={styles.dbCard}
            >
              <div class={styles.dbCardHeader}>
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
              </div>
              <div class={styles.dbCardCreated}>
                Created {formatRelativeTime(db.created_at)}
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
