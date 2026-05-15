import { useState } from 'preact/hooks';
import type { App } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Toggle from '@islands/shared/Toggle/Toggle';
import Button from '@islands/shared/Button/Button';
import { Moon, Sun } from 'lucide-preact';
import formStyles from '@styles/form.module.css';
import styles from './ghost-mode-section.module.css';

type Props = {
  app: App;
};

export default function GhostModeSection({ app }: Props) {
  const [enabled, setEnabled] = useState(app.ghost_mode_enabled);
  const [idleMinutes, setIdleMinutes] = useState(app.ghost_mode_idle_minutes);
  const [waking, setWaking] = useState(false);
  const [saving, setSaving] = useState(false);

  const isHibernating = app.ghost_mode_status === 'hibernating';

  async function handleToggle(checked: boolean) {
    setSaving(true);
    try {
      await api.updateApp(app.id, { ghost_mode_enabled: checked });
      setEnabled(checked);
      addToast('success', checked ? 'Ghost mode enabled' : 'Ghost mode disabled');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to update ghost mode');
    }
    setSaving(false);
  }

  async function handleWake() {
    setWaking(true);
    try {
      await api.wakeApp(app.id);
      addToast('success', 'App is waking up');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to wake app');
    }
    setWaking(false);
  }

  async function handleIdleChange(value: string) {
    const minutes = parseInt(value, 10);
    if (isNaN(minutes) || minutes < 1) return;
    setIdleMinutes(minutes);
    try {
      await api.updateApp(app.id, { ghost_mode_idle_minutes: minutes });
      addToast('success', 'Idle timeout updated');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to update idle timeout');
    }
  }

  return (
    <div class={styles.container}>
      <div class={styles.header}>
        <h3 class={styles.title}>
          {isHibernating ? (
            <Moon size={18} aria-hidden="true" />
          ) : (
            <Sun size={18} aria-hidden="true" />
          )}
          Ghost Mode
        </h3>
        <div class={styles.headerActions}>
          {/* a11y [WCAG 4.1.3]: status badge announced to AT */}
          <span
            class={`${styles.statusBadge} ${isHibernating ? styles.statusHibernating : styles.statusActive}`}
            role="status"
            aria-live="polite"
          >
            {isHibernating ? 'Hibernating' : 'Active'}
          </span>
          {isHibernating && (
            <Button
              variant="primary"
              size="sm"
              onClick={handleWake}
              loading={waking}
              aria-label="Wake app from hibernation"
            >
              <Sun size={14} aria-hidden="true" /> Wake
            </Button>
          )}
        </div>
      </div>

      <div class={styles.config}>
        <Toggle
          label="Enable ghost mode"
          description="Automatically hibernate the app after a period of inactivity to save resources"
          checked={enabled}
          disabled={saving}
          onChange={handleToggle}
        />

        {enabled && (
          <div class={styles.timeoutRow}>
            <label htmlFor="ghost-idle-timeout" class={formStyles.label}>
              Idle timeout
            </label>
            <input
              id="ghost-idle-timeout"
              type="number"
              min="1"
              class={`${formStyles.input} ${styles.timeoutInput}`}
              value={idleMinutes}
              onBlur={(e) => handleIdleChange((e.target as HTMLInputElement).value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleIdleChange((e.target as HTMLInputElement).value);
              }}
              onInput={(e) => setIdleMinutes(parseInt((e.target as HTMLInputElement).value, 10) || 0)}
            />
            <span class={styles.timeoutUnit}>minutes</span>
          </div>
        )}
      </div>
    </div>
  );
}
