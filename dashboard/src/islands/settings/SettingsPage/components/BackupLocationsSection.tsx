import { useState } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
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
            <Input
              label="Name"
              name="bk-name"
              id="bk-name"
              value={newBackup.name}
              onChange={v => setNewBackup(p => ({ ...p, name: v }))}
              placeholder="Primary backups"
            />
            <Input
              label="S3/R2 Bucket"
              name="bk-bucket"
              id="bk-bucket"
              value={newBackup.bucket}
              onChange={v => setNewBackup(p => ({ ...p, bucket: v }))}
              placeholder="my-backup-bucket"
            />
          </div>
          <div class={formStyles.fieldRow}>
            <Input
              label="Endpoint"
              name="bk-endpoint"
              id="bk-endpoint"
              mono
              value={newBackup.endpoint}
              onChange={v => setNewBackup(p => ({ ...p, endpoint: v }))}
              placeholder="https://acct-id.r2.cloudflarestorage.com"
            />
            <Input
              label="Region"
              name="bk-region"
              id="bk-region"
              value={newBackup.region}
              onChange={v => setNewBackup(p => ({ ...p, region: v }))}
              placeholder="auto"
            />
          </div>
          <div class={formStyles.fieldRow}>
            <Input
              label="Access Key"
              name="bk-access"
              id="bk-access"
              mono
              value={newBackup.access_key}
              onChange={v => setNewBackup(p => ({ ...p, access_key: v }))}
            />
            <Input
              label="Secret Key"
              name="bk-secret"
              id="bk-secret"
              type="password"
              mono
              value={newBackup.secret_key}
              onChange={v => setNewBackup(p => ({ ...p, secret_key: v }))}
            />
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
