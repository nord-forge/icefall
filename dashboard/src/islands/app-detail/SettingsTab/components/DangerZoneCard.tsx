import Button from '@islands/shared/Button/Button';
import { AlertTriangle, Square, Play, RotateCw, Trash2 } from 'lucide-preact';
import styles from '../settings-tab.module.css';

type Props = {
  confirmDelete: boolean;
  deleting: boolean;
  stopping: boolean;
  starting: boolean;
  restarting: boolean;
  onStart: () => void;
  onRestart: () => void;
  onStop: () => void;
  onDelete: () => void;
  onConfirmDeleteToggle: (v: boolean) => void;
};

export default function DangerZoneCard({
  confirmDelete,
  deleting,
  stopping,
  starting,
  restarting,
  onStart,
  onRestart,
  onStop,
  onDelete,
  onConfirmDeleteToggle,
}: Props) {
  return (
    <div class={styles.dangerCard}>
      <h2 class={styles.dangerTitle}>
        <AlertTriangle size={18} /> Danger Zone
      </h2>

      <div class={styles.dangerRowBorder}>
        <div>
          <p class={styles.dangerLabel}>Application Controls</p>
          <p class={styles.dangerDescription}>Start, stop, or restart all containers for this application.</p>
        </div>
        <div class={styles.confirmActions}>
          <Button variant="secondary" onClick={onStart} loading={starting}>
            <Play size={14} /> Start
          </Button>
          <Button variant="secondary" onClick={onRestart} loading={restarting}>
            <RotateCw size={14} /> Restart
          </Button>
          <Button variant="danger" onClick={onStop} loading={stopping}>
            <Square size={14} /> Stop
          </Button>
        </div>
      </div>

      <div class={styles.dangerRow}>
        <div>
          <p class={styles.dangerLabel}>Delete Application</p>
          <p class={styles.dangerDescription}>Deleting this app will remove all deploys, domains, and environment variables. This action cannot be undone.</p>
        </div>
        {confirmDelete ? (
          <div class={styles.confirmActions}>
            <Button variant="ghost" onClick={() => onConfirmDeleteToggle(false)}>Cancel</Button>
            <Button variant="danger" onClick={onDelete} loading={deleting}>
              <Trash2 size={14} /> Confirm Delete
            </Button>
          </div>
        ) : (
          <Button variant="danger" onClick={() => onConfirmDeleteToggle(true)}>
            <Trash2 size={14} /> Delete App
          </Button>
        )}
      </div>
    </div>
  );
}
