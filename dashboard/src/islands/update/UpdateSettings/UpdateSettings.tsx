import { useEffect, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { RefreshCw, Save, CheckCircle2, XCircle } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import { api } from '@lib/api';
import { $updateInfo, $updateDialogOpen } from '@stores/update';
import { addToast } from '@stores/toast';
import type { UpdateInfo } from '@stores/update';
import formStyles from '@styles/form.module.css';
import styles from './update-settings.module.css';

type UpdatePreferences = {
  channel: 'stable' | 'beta';
  auto_update_enabled: boolean;
  auto_update_window_start: string;
  auto_update_window_end: string;
  auto_update_notify_before_minutes: number;
};

type HistoryEntry = {
  version: string;
  date: string;
  duration_secs: number;
  status: 'success' | 'failed' | 'rolled_back';
};

function formatDuration(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  const mins = Math.floor(secs / 60);
  const remaining = Math.round(secs % 60);
  return remaining > 0 ? `${mins}m ${remaining}s` : `${mins}m`;
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

export default function UpdateSettings() {
  const info = useStore($updateInfo);
  const [checking, setChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<'up_to_date' | 'available' | null>(null);
  const [prefs, setPrefs] = useState<UpdatePreferences>({
    channel: 'stable',
    auto_update_enabled: false,
    auto_update_window_start: '03:00',
    auto_update_window_end: '05:00',
    auto_update_notify_before_minutes: 30,
  });
  const [saving, setSaving] = useState(false);
  const [history, setHistory] = useState<HistoryEntry[]>([]);

  useEffect(() => {
    api.getUpdatePreferences().then((res) => {
      if (res.data) {
        setPrefs((prev) => ({
          ...prev,
          channel: res.data.channel ?? prev.channel,
          auto_update_enabled: res.data.auto_update_enabled ?? prev.auto_update_enabled,
          auto_update_window_start: res.data.auto_update_window_start ?? prev.auto_update_window_start,
          auto_update_window_end: res.data.auto_update_window_end ?? prev.auto_update_window_end,
          auto_update_notify_before_minutes: res.data.auto_update_notify_before_minutes ?? prev.auto_update_notify_before_minutes,
        }));
      }
    }).catch(() => {});

    api.getUpdateHistory().then((res) => {
      setHistory(res.data || []);
    }).catch(() => {});

    if (!info) {
      api.checkForUpdate().then((res) => {
        $updateInfo.set(res.data as UpdateInfo);
      }).catch(() => {});
    }
  }, []);

  async function handleCheck() {
    setChecking(true);
    setCheckResult(null);
    try {
      const res = await api.checkForUpdate();
      const data = res.data as UpdateInfo;
      $updateInfo.set(data);
      setCheckResult(data.available ? 'available' : 'up_to_date');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to check for updates');
    }
    setChecking(false);
  }

  async function handleSavePrefs() {
    setSaving(true);
    try {
      await api.updateUpdatePreferences(prefs);
      addToast('success', 'Update preferences saved');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to save preferences');
    }
    setSaving(false);
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}>
          <RefreshCw size={18} aria-hidden="true" class={checking ? styles.spinIcon : ''} />
          Updates
        </h2>
      </div>

      {/* Current version + check */}
      <div class={styles.versionRow}>
        <div>
          <p class={styles.versionLabel}>Current Version</p>
          <p class={styles.versionValue}>{info?.current_version || 'loading...'}</p>
        </div>
        <Button variant="secondary" onClick={handleCheck} disabled={checking} loading={checking}>
          <RefreshCw size={14} aria-hidden="true" class={checking ? styles.spinIcon : ''} />
          Check for updates
        </Button>
      </div>

      {checkResult && (
        <div class={styles.checkResult} role="status" aria-live="polite">
          {checkResult === 'up_to_date' ? (
            <span class={styles.checkResultSuccess}>You are on the latest version.</span>
          ) : (
            <span class={styles.checkResultAvailable}>
              Version {info?.latest_version} is available.{' '}
              <button
                type="button"
                class={styles.linkButton}
                onClick={() => $updateDialogOpen.set(true)}
              >
                View details
              </button>
            </span>
          )}
        </div>
      )}

      {/* Update channel */}
      <div style={{ marginTop: 'var(--space-5)' }}>
        <label class={formStyles.label}>Update Channel</label>
        <div class={styles.radioGroup} role="radiogroup" aria-label="Update channel">
          <label class={styles.radioLabel}>
            <input
              type="radio"
              name="update-channel"
              value="stable"
              checked={prefs.channel === 'stable'}
              onChange={() => setPrefs((p) => ({ ...p, channel: 'stable' }))}
            />
            Stable
          </label>
          <p class={styles.radioHint}>Recommended. Receives tested, production-ready releases.</p>
          <label class={styles.radioLabel}>
            <input
              type="radio"
              name="update-channel"
              value="beta"
              checked={prefs.channel === 'beta'}
              onChange={() => setPrefs((p) => ({ ...p, channel: 'beta' }))}
            />
            Beta
          </label>
          <p class={styles.radioHint}>Early access to new features. May contain bugs.</p>
        </div>
      </div>

      {/* Auto-update toggle */}
      <div style={{ marginTop: 'var(--space-5)' }}>
        <label htmlFor="auto-update-toggle" class={formStyles.label}>Automatic Updates</label>
        <div class={styles.toggleRow}>
          <button
            id="auto-update-toggle"
            type="button"
            role="switch"
            aria-checked={prefs.auto_update_enabled}
            class={`${styles.toggle} ${prefs.auto_update_enabled ? styles.toggleOn : ''}`}
            onClick={() => setPrefs((p) => ({ ...p, auto_update_enabled: !p.auto_update_enabled }))}
          >
            <span class={styles.toggleKnob}>
              <svg class={styles.toggleIcon} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
                <path class={styles.toggleCheck} d="M2.5 5 L4.5 7 L7.5 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
                <path class={styles.toggleCross} d="M3 3 L7 7 M7 3 L3 7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              </svg>
            </span>
          </button>
          <span class={styles.toggleLabel}>
            {prefs.auto_update_enabled ? 'On' : 'Off'}
          </span>
        </div>
        <p class={formStyles.hint}>
          When enabled, updates are applied automatically during the maintenance window.
        </p>

        {prefs.auto_update_enabled && (
          <div class={styles.windowRow}>
            <div>
              <label htmlFor="mw-start" class={formStyles.label}>Start</label>
              <input
                id="mw-start"
                type="time"
                class={`${formStyles.input} ${styles.timeInput}`}
                value={prefs.auto_update_window_start}
                onInput={(e) =>
                  setPrefs((p) => ({
                    ...p,
                    auto_update_window_start: (e.target as HTMLInputElement).value,
                  }))
                }
              />
            </div>
            <div>
              <label htmlFor="mw-end" class={formStyles.label}>End</label>
              <input
                id="mw-end"
                type="time"
                class={`${formStyles.input} ${styles.timeInput}`}
                value={prefs.auto_update_window_end}
                onInput={(e) =>
                  setPrefs((p) => ({
                    ...p,
                    auto_update_window_end: (e.target as HTMLInputElement).value,
                  }))
                }
              />
            </div>
            <p class={styles.windowHint}>Server local time</p>
          </div>
        )}
      </div>

      <div class={styles.saveRow}>
        <Button variant="primary" onClick={handleSavePrefs} loading={saving}>
          <Save size={14} aria-hidden="true" /> Save Preferences
        </Button>
      </div>

      {/* Update history */}
      <h3 class={styles.historyHeading}>Update History</h3>
      {history.length === 0 ? (
        <p class={styles.emptyText}>No update history available.</p>
      ) : (
        <table class={styles.historyTable}>
          <thead>
            <tr>
              <th scope="col">Version</th>
              <th scope="col">Date</th>
              <th scope="col">Duration</th>
              <th scope="col">Status</th>
            </tr>
          </thead>
          <tbody>
            {history.map((entry, i) => (
              <tr key={i}>
                <td class={styles.monoText}>{entry.version}</td>
                <td>{formatDate(entry.date)}</td>
                <td class={styles.monoText}>{formatDuration(entry.duration_secs)}</td>
                <td>
                  <span
                    class={`${styles.statusBadge} ${
                      entry.status === 'success'
                        ? styles.statusSuccess
                        : entry.status === 'failed'
                          ? styles.statusFailed
                          : styles.statusRolledBack
                    }`}
                  >
                    {entry.status === 'success' && <CheckCircle2 size={12} aria-hidden="true" />}
                    {entry.status === 'failed' && <XCircle size={12} aria-hidden="true" />}
                    {entry.status === 'success'
                      ? 'Success'
                      : entry.status === 'failed'
                        ? 'Failed'
                        : 'Rolled back'}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
