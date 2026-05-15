import { useState, useEffect } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { CleanupRun } from '@lib/types';
import { Trash2, Play } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from './server-cleanup-card.module.css';

type Props = {
  serverId: string;
};

function formatRunTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1
  );
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export default function ServerCleanupCard({ serverId }: Props) {
  const [lastRun, setLastRun] = useState<CleanupRun | null>(null);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        const { data } = await api.listCleanupHistory();
        if (active && data.length > 0) {
          setLastRun(data[0]);
        }
      } catch {
        // Keep null
      }
      if (active) setLoading(false);
    }

    load();
    return () => { active = false; };
  }, [serverId]);

  async function handleRunCleanup() {
    setRunning(true);
    try {
      await api.runCleanup();
      addToast('info', 'Cleanup started on server');
      // Refresh last run after delay
      setTimeout(async () => {
        try {
          const { data } = await api.listCleanupHistory();
          if (data.length > 0) setLastRun(data[0]);
        } catch {}
      }, 3000);
    } catch (err: any) {
      addToast('error', err.message || 'Failed to start cleanup');
    }
    setRunning(false);
  }

  return (
    <div class={styles.container}>
      <h3 class={styles.title}>
        <Trash2 size={16} aria-hidden="true" />
        Container Cleanup
      </h3>

      {loading ? (
        <div class={styles.statusRow}>
          <span class={styles.statusLabel}>Loading...</span>
        </div>
      ) : lastRun ? (
        <>
          <div class={styles.statusRow}>
            <span class={styles.statusLabel}>Last run</span>
            <span class={styles.statusValue}>
              {formatRunTime(lastRun.started_at)}
            </span>
          </div>
          <div class={styles.statusRow}>
            <span class={styles.statusLabel}>Result</span>
            <span class={styles.statusValue}>
              {lastRun.status === 'completed'
                ? `Freed ${formatBytes(lastRun.freed_bytes)}`
                : lastRun.status === 'running'
                  ? 'In progress'
                  : `Failed: ${lastRun.error || 'Unknown error'}`}
            </span>
          </div>
        </>
      ) : (
        <div class={styles.statusRow}>
          <span class={styles.statusLabel}>No cleanup runs recorded</span>
        </div>
      )}

      <Button
        variant="secondary"
        onClick={handleRunCleanup}
        loading={running}
      >
        <Play size={14} aria-hidden="true" /> Run cleanup
      </Button>
      <p class={styles.feedbackText} role="status" aria-live="polite">
        {running ? 'Running cleanup...' : ''}
      </p>
    </div>
  );
}
