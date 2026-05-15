import { useState, useEffect } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { CleanupSchedule, CleanupRun } from '@lib/types';
import { Trash2, Play, Clock } from 'lucide-preact';
import Toggle from '@islands/shared/Toggle/Toggle';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  onSaveMessage: (msg: string) => void;
};

type SchedulePreset = 'daily' | 'weekly' | 'custom';

const PRESET_CRONS: Record<Exclude<SchedulePreset, 'custom'>, string> = {
  daily: '0 2 * * *',
  weekly: '0 2 * * 0',
};

function detectPreset(cron: string): SchedulePreset {
  if (cron === PRESET_CRONS.daily) return 'daily';
  if (cron === PRESET_CRONS.weekly) return 'weekly';
  return 'custom';
}

function formatRunTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export default function ContainerCleanupSection({ onSaveMessage }: Props) {
  const [schedule, setSchedule] = useState<CleanupSchedule>({
    cron: '0 2 * * *',
    disk_threshold_percent: 80,
    dangling_images: true,
    unused_images: false,
    stopped_containers: false,
    stopped_container_age_hours: 24,
    volumes: false,
    networks: false,
    enabled: false,
  });
  const [preset, setPreset] = useState<SchedulePreset>('daily');
  const [history, setHistory] = useState<CleanupRun[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [runningNow, setRunningNow] = useState(false);

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        const [scheduleRes, historyRes] = await Promise.all([
          api.getCleanupSchedule(),
          api.listCleanupHistory(),
        ]);
        if (!active) return;
        setSchedule(scheduleRes.data);
        setPreset(detectPreset(scheduleRes.data.cron));
        setHistory(historyRes.data);
      } catch {
        // Keep defaults
      }
      if (active) setLoading(false);
    }

    load();
    return () => { active = false; };
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      const { data } = await api.updateCleanupSchedule(schedule);
      setSchedule(data);
      onSaveMessage('Cleanup schedule saved');
      addToast('success', 'Cleanup schedule updated');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to save cleanup schedule');
    }
    setSaving(false);
  }

  async function handleRunNow() {
    setRunningNow(true);
    try {
      await api.runCleanup();
      addToast('info', 'Cleanup started');
      // Refresh history after a short delay
      setTimeout(async () => {
        try {
          const { data } = await api.listCleanupHistory();
          setHistory(data);
        } catch {}
      }, 2000);
    } catch (err: any) {
      addToast('error', err.message || 'Failed to start cleanup');
    }
    setRunningNow(false);
  }

  function handlePresetChange(newPreset: SchedulePreset) {
    setPreset(newPreset);
    if (newPreset !== 'custom') {
      setSchedule((s) => ({ ...s, cron: PRESET_CRONS[newPreset] }));
    }
  }

  function updateSchedule(partial: Partial<CleanupSchedule>) {
    setSchedule((s) => ({ ...s, ...partial }));
  }

  if (loading) {
    return (
      <div class={styles.section}>
        <h2 class={styles.sectionHeading}>
          <Trash2 size={18} aria-hidden="true" /> Container Cleanup
        </h2>
        <p class={styles.emptyText} role="status" aria-live="polite">
          Loading cleanup settings...
        </p>
      </div>
    );
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}>
          <Trash2 size={18} aria-hidden="true" /> Container Cleanup
        </h2>
        <Button
          variant="secondary"
          onClick={handleRunNow}
          loading={runningNow}
        >
          <Play size={14} aria-hidden="true" /> Run now
        </Button>
      </div>

      <Toggle
        label="Enable scheduled cleanup"
        description="Automatically remove unused containers and images on a schedule."
        checked={schedule.enabled}
        onChange={(checked) => updateSchedule({ enabled: checked })}
      />

      {schedule.enabled && (
        <>
          {/* Schedule presets */}
          <div style={{ marginTop: 'var(--space-4)' }}>
            <label class={formStyles.label}>Schedule</label>
            <div style={{ display: 'flex', gap: 'var(--space-2)', marginTop: 'var(--space-2)' }}>
              {(['daily', 'weekly', 'custom'] as SchedulePreset[]).map((p) => (
                <Button
                  key={p}
                  variant={preset === p ? 'primary' : 'secondary'}
                  size="sm"
                  onClick={() => handlePresetChange(p)}
                >
                  {p === 'daily' && 'Daily 2am'}
                  {p === 'weekly' && 'Weekly Sunday 2am'}
                  {p === 'custom' && 'Custom'}
                </Button>
              ))}
            </div>
          </div>

          {/* Custom cron input */}
          {preset === 'custom' && (
            <div style={{ marginTop: 'var(--space-3)' }}>
              <Input
                label="Cron expression"
                name="cleanup-cron"
                id="cleanup-cron"
                mono
                value={schedule.cron}
                onChange={(v) => updateSchedule({ cron: v })}
                placeholder="0 2 * * *"
                helpText="Standard 5-field cron syntax (minute hour day month weekday)."
              />
            </div>
          )}

          {/* Disk threshold slider */}
          <div style={{ marginTop: 'var(--space-4)' }}>
            <label htmlFor="cleanup-threshold" class={formStyles.label}>
              Disk threshold: {schedule.disk_threshold_percent}%
            </label>
            <input
              id="cleanup-threshold"
              type="range"
              min="50"
              max="95"
              value={schedule.disk_threshold_percent}
              onInput={(e) =>
                updateSchedule({
                  disk_threshold_percent: parseInt(
                    (e.target as HTMLInputElement).value,
                    10
                  ),
                })
              }
              style={{ width: '100%', marginTop: 'var(--space-2)' }}
              aria-valuemin={50}
              aria-valuemax={95}
              aria-valuenow={schedule.disk_threshold_percent}
              aria-valuetext={`${schedule.disk_threshold_percent}%`}
            />
            <span class={styles.hint}>
              Cleanup triggers when disk usage exceeds this threshold.
            </span>
          </div>

          {/* Cleanup targets */}
          <div style={{ marginTop: 'var(--space-4)' }}>
            <label class={formStyles.label}>Cleanup targets</label>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 'var(--space-3)',
                marginTop: 'var(--space-2)',
              }}
            >
              <Toggle
                label="Dangling images"
                description="Remove images without tags."
                checked={schedule.dangling_images}
                onChange={(v) => updateSchedule({ dangling_images: v })}
              />
              <Toggle
                label="Unused images"
                description="Remove images not used by any container."
                checked={schedule.unused_images}
                onChange={(v) => updateSchedule({ unused_images: v })}
              />
              <Toggle
                label="Stopped containers"
                description="Remove containers that have been stopped."
                checked={schedule.stopped_containers}
                onChange={(v) => updateSchedule({ stopped_containers: v })}
              />
              {schedule.stopped_containers && (
                <div style={{ paddingLeft: 'var(--space-6)', maxWidth: '160px' }}>
                  <Input
                    label="Minimum age (hours)"
                    name="cleanup-container-age"
                    id="cleanup-container-age"
                    type="number"
                    min={1}
                    value={String(schedule.stopped_container_age_hours)}
                    onChange={(v) =>
                      updateSchedule({
                        stopped_container_age_hours: parseInt(v, 10) || 1,
                      })
                    }
                    helpText="Only remove containers stopped for at least this many hours."
                  />
                </div>
              )}
              <Toggle
                label="Unused volumes"
                description="Remove volumes not attached to any container."
                checked={schedule.volumes}
                onChange={(v) => updateSchedule({ volumes: v })}
              />
              <Toggle
                label="Unused networks"
                description="Remove networks not used by any container."
                checked={schedule.networks}
                onChange={(v) => updateSchedule({ networks: v })}
              />
            </div>
          </div>

          <div class={styles.saveRow}>
            <Button
              variant="primary"
              onClick={handleSave}
              loading={saving}
            >
              Save cleanup schedule
            </Button>
          </div>
        </>
      )}

      {/* Cleanup history */}
      {history.length > 0 && (
        <div style={{ marginTop: 'var(--space-5)' }}>
          <h3 class={styles.subHeading}>
            <Clock size={14} aria-hidden="true" /> Recent cleanup runs
          </h3>
          <div class={styles.itemList}>
            {history.slice(0, 10).map((run) => (
              <div key={run.id} class={styles.itemRow}>
                <div class={styles.itemInfo}>
                  <span class={styles.itemLabel}>
                    {formatRunTime(run.started_at)}
                  </span>
                  <span class={styles.itemMeta}>
                    {run.status === 'completed'
                      ? `Freed ${formatBytes(run.freed_bytes)}, removed ${run.removed_items} items`
                      : run.status === 'running'
                        ? 'In progress...'
                        : `Failed: ${run.error || 'Unknown error'}`}
                  </span>
                </div>
                <div class={styles.itemActions}>
                  <span
                    class={
                      run.status === 'completed'
                        ? styles.statusIconSuccess
                        : run.status === 'running'
                          ? styles.statusIconRunning
                          : styles.statusIconError
                    }
                    aria-label={run.status}
                  >
                    {run.status === 'completed'
                      ? 'Done'
                      : run.status === 'running'
                        ? 'Running'
                        : 'Failed'}
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
