import { useState, useEffect } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import { Save, HardDrive, Play, CheckCircle, XCircle, Clock } from 'lucide-preact';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

type BackupHistoryEntry = {
  id: string;
  filename: string;
  size_bytes: number;
  status: string;
  error_message: string | null;
  s3_key: string | null;
  started_at: string;
  finished_at: string | null;
};

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const val = bytes / Math.pow(1024, i);
  return val < 10 ? `${val.toFixed(1)} ${units[i]}` : `${Math.round(val)} ${units[i]}`;
}

function formatRelativeTime(iso: string): string {
  try {
    const date = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHrs = Math.floor(diffMins / 60);
    if (diffHrs < 24) return `${diffHrs}h ago`;
    const diffDays = Math.floor(diffHrs / 24);
    return `${diffDays}d ago`;
  } catch { return iso; }
}

type Props = {
  onSaveMessage: (msg: string) => void;
};

export default function InstanceBackupSection({ onSaveMessage }: Props) {
  const [ibEnabled, setIbEnabled] = useState(false);
  const [ibSchedule, setIbSchedule] = useState('daily');
  const [ibRetention, setIbRetention] = useState(7);
  const [ibHistory, setIbHistory] = useState<BackupHistoryEntry[]>([]);
  const [ibSaving, setIbSaving] = useState(false);
  const [ibTriggering, setIbTriggering] = useState(false);

  useEffect(() => {
    fetch('/api/v1/settings/instance-backup', { credentials: 'same-origin' }).then(r => r.json()).then(d => {
      if (d.data) {
        setIbEnabled(d.data.enabled);
        setIbSchedule(d.data.cron_schedule || 'daily');
        setIbRetention(d.data.retention_count ?? 7);
      }
    }).catch(() => {});

    fetch('/api/v1/settings/instance-backup/history', { credentials: 'same-origin' }).then(r => r.json()).then(d => {
      setIbHistory(d.data || []);
    }).catch(() => {});
  }, []);

  async function saveConfig() {
    setIbSaving(true);
    try {
      const res = await fetch('/api/v1/settings/instance-backup', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'same-origin',
        body: JSON.stringify({ enabled: ibEnabled, cron_schedule: ibSchedule, retention_count: ibRetention }),
      });
      const d = await res.json();
      if (d.data) {
        setIbEnabled(d.data.enabled);
        setIbSchedule(d.data.cron_schedule);
        setIbRetention(d.data.retention_count);
      }
      onSaveMessage('Instance backup settings saved');
    } catch { onSaveMessage('Failed to save instance backup settings'); }
    setIbSaving(false);
  }

  async function triggerBackup() {
    setIbTriggering(true);
    try {
      await fetch('/api/v1/settings/instance-backup/trigger', {
        method: 'POST',
        credentials: 'same-origin',
      });
      onSaveMessage('Instance backup triggered');
      setTimeout(async () => {
        try {
          const res = await fetch('/api/v1/settings/instance-backup/history', { credentials: 'same-origin' });
          const d = await res.json();
          setIbHistory(d.data || []);
        } catch {}
      }, 2000);
    } catch { onSaveMessage('Failed to trigger instance backup'); }
    setIbTriggering(false);
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}><HardDrive size={18} aria-hidden="true" /> Instance Backup</h2>
        <Button variant="secondary" onClick={triggerBackup} loading={ibTriggering}>
          <Play size={14} aria-hidden="true" /> Backup Now
        </Button>
      </div>
      <p class={styles.hint} style={{ marginTop: 0, marginBottom: 'var(--space-4)' }}>
        Full instance backup including database, config, volumes, and managed database dumps. Uploaded to your configured S3 location or stored locally.
      </p>

      <div class={formStyles.fieldRow}>
        <div>
          {/* a11y [1.3.1]: label explicitly associated with toggle via htmlFor */}
          <label htmlFor="ib-enabled" class={formStyles.label}>Enable Scheduled Backups</label>
          <div class={styles.toggleRow}>
            <button
              id="ib-enabled"
              type="button"
              role="switch"
              aria-checked={ibEnabled}
              class={`${styles.toggle} ${ibEnabled ? styles.toggleOn : ''}`}
              onClick={() => setIbEnabled(!ibEnabled)}
            >
              <span class={styles.toggleKnob}>
                {/* a11y [WCAG 1.4.1]: shape cue inside knob — not color alone */}
                <svg class={styles.toggleIcon} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
                  <path class={styles.toggleCheck} d="M2.5 5 L4.5 7 L7.5 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
                  <path class={styles.toggleCross} d="M3 3 L7 7 M7 3 L3 7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
                </svg>
              </span>
            </button>
            <span class={styles.toggleLabel}>{ibEnabled ? 'On' : 'Off'}</span>
          </div>
        </div>
        <div>
          <label htmlFor="ib-schedule" class={formStyles.label}>Schedule</label>
          <Select
            id="ib-schedule"
            options={[
              { value: 'daily', label: 'Daily' },
              { value: 'weekly', label: 'Weekly' },
              { value: 'monthly', label: 'Monthly' },
            ]}
            value={ibSchedule}
            onChange={setIbSchedule}
            fullWidth
          />
        </div>
        <Input
          label="Retention Count"
          name="ib-retention"
          id="ib-retention"
          type="number"
          min={1}
          max={365}
          value={String(ibRetention)}
          onChange={(v) => {
            const val = parseInt(v, 10);
            if (!isNaN(val)) setIbRetention(val);
          }}
          helpText="Number of backups to keep before old ones are removed."
        />
      </div>

      <div class={styles.saveRow}>
        <Button variant="primary" onClick={saveConfig} loading={ibSaving}>
          <Save size={14} aria-hidden="true" /> Save Backup Settings
        </Button>
      </div>

      {ibHistory.length > 0 && (
        <div style={{ marginTop: 'var(--space-5)' }}>
          <h3 class={styles.subHeading}>Recent Backups</h3>
          <div class={styles.itemList}>
            {ibHistory.slice(0, 10).map(b => (
              <div key={b.id} class={styles.itemRow}>
                <div class={styles.itemInfo}>
                  <span class={styles.itemLabel}>
                    {b.status === 'completed' && <CheckCircle size={14} aria-hidden="true" class={styles.statusIconSuccess} />}
                    {b.status === 'failed' && <XCircle size={14} aria-hidden="true" class={styles.statusIconError} />}
                    {b.status === 'running' && <Clock size={14} aria-hidden="true" class={styles.statusIconRunning} />}
                    {' '}{b.filename}
                  </span>
                  <span class={styles.itemMeta}>
                    {formatRelativeTime(b.started_at)}
                    {b.status === 'completed' && b.size_bytes > 0 ? ` · ${formatBytes(b.size_bytes)}` : ''}
                    {b.status === 'failed' && b.error_message ? ` · ${b.error_message}` : ''}
                    {b.status === 'running' ? ' · In progress...' : ''}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
