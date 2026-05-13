import { useState } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import { Database, Plus, Trash2 } from 'lucide-preact';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

type BackupLocation = {
  id: string;
  name: string;
  bucket: string;
  endpoint: string;
  region: string;
};

type Props = {
  onSaveMessage: (msg: string) => void;
};

export default function BackupLocationsSection({ onSaveMessage }: Props) {
  const [backups, setBackups] = useState<BackupLocation[]>([]);
  const [showAddBackup, setShowAddBackup] = useState(false);
  const [newBackup, setNewBackup] = useState({ name: '', bucket: '', endpoint: '', region: '', access_key: '', secret_key: '' });
  const [saving, setSaving] = useState(false);

  async function addBackupLocation() {
    setSaving(true);
    setBackups(prev => [...prev, { id: crypto.randomUUID(), ...newBackup }]);
    setShowAddBackup(false);
    setNewBackup({ name: '', bucket: '', endpoint: '', region: '', access_key: '', secret_key: '' });
    onSaveMessage('Backup location added');
    setSaving(false);
  }

  function removeBackup(id: string) {
    setBackups(prev => prev.filter(b => b.id !== id));
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}><Database size={18} aria-hidden="true" /> Backup Locations</h2>
        <Button variant="secondary" onClick={() => setShowAddBackup(true)}>
          <Plus size={14} aria-hidden="true" /> Add Location
        </Button>
      </div>

      {showAddBackup && (
        <div class={styles.addCard}>
          <div class={formStyles.fieldRow}>
            <div>
              <label htmlFor="bk-name" class={formStyles.label}>Name</label>
              <input id="bk-name" class={formStyles.input} value={newBackup.name} onInput={e => setNewBackup(p => ({ ...p, name: (e.target as HTMLInputElement).value }))} placeholder="Primary backups" />
            </div>
            <div>
              <label htmlFor="bk-bucket" class={formStyles.label}>S3/R2 Bucket</label>
              <input id="bk-bucket" class={formStyles.input} value={newBackup.bucket} onInput={e => setNewBackup(p => ({ ...p, bucket: (e.target as HTMLInputElement).value }))} placeholder="my-backup-bucket" />
            </div>
          </div>
          <div class={formStyles.fieldRow}>
            <div>
              <label htmlFor="bk-endpoint" class={formStyles.label}>Endpoint</label>
              <input id="bk-endpoint" class={formStyles.inputMono} value={newBackup.endpoint} onInput={e => setNewBackup(p => ({ ...p, endpoint: (e.target as HTMLInputElement).value }))} placeholder="https://acct-id.r2.cloudflarestorage.com" />
            </div>
            <div>
              <label htmlFor="bk-region" class={formStyles.label}>Region</label>
              <input id="bk-region" class={formStyles.input} value={newBackup.region} onInput={e => setNewBackup(p => ({ ...p, region: (e.target as HTMLInputElement).value }))} placeholder="auto" />
            </div>
          </div>
          <div class={formStyles.fieldRow}>
            <div>
              <label htmlFor="bk-access" class={formStyles.label}>Access Key</label>
              <input id="bk-access" class={formStyles.inputMono} value={newBackup.access_key} onInput={e => setNewBackup(p => ({ ...p, access_key: (e.target as HTMLInputElement).value }))} />
            </div>
            <div>
              <label htmlFor="bk-secret" class={formStyles.label}>Secret Key</label>
              <input id="bk-secret" class={formStyles.inputMono} type="password" autoComplete="off" value={newBackup.secret_key} onInput={e => setNewBackup(p => ({ ...p, secret_key: (e.target as HTMLInputElement).value }))} />
            </div>
          </div>
          <div class={styles.addCardActions}>
            <Button variant="ghost" onClick={() => setShowAddBackup(false)}>Cancel</Button>
            <Button variant="primary" onClick={addBackupLocation} loading={saving} disabled={!newBackup.name.trim() || !newBackup.bucket.trim()}>Add Location</Button>
          </div>
        </div>
      )}

      {backups.length === 0 && !showAddBackup ? (
        <p class={styles.emptyText}>No backup locations configured. Backups are stored locally by default.</p>
      ) : (
        <div class={styles.itemList}>
          {backups.map(b => (
            <div key={b.id} class={styles.itemRow}>
              <div class={styles.itemInfo}>
                <span class={styles.itemLabel}>{b.name}</span>
                <span class={styles.itemMeta}>{b.bucket}{b.endpoint ? ` · ${b.endpoint}` : ''}{b.region ? ` · ${b.region}` : ''}</span>
              </div>
              <div class={styles.itemActions}>
                <button type="button" class={styles.iconButton} onClick={() => removeBackup(b.id)} aria-label={`Remove ${b.name}`}>
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
