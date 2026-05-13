import type { Server } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import Select from '@islands/shared/Select/Select';
import { ArrowRightLeft } from 'lucide-preact';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  appName: string;
  serverId: string;
  currentServer: Server | undefined;
  availableTargets: Server[];
  migrateTarget: string;
  showMigrateDialog: boolean;
  migrateAck: boolean;
  migrating: boolean;
  onMigrateTargetChange: (v: string) => void;
  onShowMigrateDialog: () => void;
  onCloseMigrateDialog: () => void;
  onMigrateAckChange: (v: boolean) => void;
  onMigrate: () => void;
};

export default function ServerPlacementCard({
  appName,
  serverId,
  currentServer,
  availableTargets,
  migrateTarget,
  showMigrateDialog,
  migrateAck,
  migrating,
  onMigrateTargetChange,
  onShowMigrateDialog,
  onCloseMigrateDialog,
  onMigrateAckChange,
  onMigrate,
}: Props) {
  return (
    <>
      <div class={styles.card}>
        <h2 class={styles.sectionTitle}>
          <ArrowRightLeft size={18} aria-hidden="true" /> Server placement
        </h2>
        <div class={styles.serverPlacement}>
          <div class={styles.currentServer}>
            <span class={styles.serverLabel}>Current server</span>
            <a href={`/servers/${serverId}`} class={styles.serverValue}>
              {currentServer?.name || serverId || 'Control plane'}
            </a>
          </div>
          {availableTargets.length > 0 && (
            <div class={styles.migrateRow}>
              <div>
                <label htmlFor="migrate-target" class={formStyles.label}>Migrate to</label>
                <Select
                  id="migrate-target"
                  options={availableTargets.map((s) => ({ value: s.id, label: `${s.name} (${s.host})` }))}
                  value={migrateTarget}
                  onChange={onMigrateTargetChange}
                  placeholder="Select a server..."
                  fullWidth
                />
              </div>
              <Button
                variant="secondary"
                onClick={onShowMigrateDialog}
                disabled={!migrateTarget}
              >
                <ArrowRightLeft size={14} /> Migrate app
              </Button>
            </div>
          )}
        </div>
      </div>

      {showMigrateDialog && (
        <div class={styles.migrateDialogBackdrop} onClick={onCloseMigrateDialog}>
          <div
            class={styles.migrateDialog}
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="migrate-title"
            aria-describedby="migrate-desc"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 id="migrate-title" class={styles.migrateDialogTitle}>Migrate app?</h2>
            <p id="migrate-desc" class={styles.migrateDialogDesc}>
              This will redeploy "{appName}" from <strong>{currentServer?.name || 'current server'}</strong> to <strong>{availableTargets.find(s => s.id === migrateTarget)?.name}</strong>.
            </p>
            <label class={styles.migrateCheckbox}>
              <input
                type="checkbox"
                checked={migrateAck}
                onChange={(e) => onMigrateAckChange((e.target as HTMLInputElement).checked)}
              />
              I understand that volume data will not be migrated
            </label>
            <div class={styles.migrateDialogActions}>
              <Button variant="ghost" onClick={() => { onCloseMigrateDialog(); onMigrateAckChange(false); }}>Cancel</Button>
              <Button variant="primary" onClick={onMigrate} loading={migrating} disabled={!migrateAck}>
                Migrate
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
