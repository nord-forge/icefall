import { useState } from 'preact/hooks';
import { Settings2 } from 'lucide-preact';
import Select from '@islands/shared/Select/Select';
import styles from '../profile-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  preferences: Record<string, unknown>;
  onUpdatePreference: (update: Record<string, unknown>) => Promise<void>;
};

export default function PreferencesSection({ preferences, onUpdatePreference }: Props) {
  const [saving, setSaving] = useState(false);

  async function savePreference(update: Record<string, unknown>) {
    setSaving(true);
    try { await onUpdatePreference(update); } catch {}
    setSaving(false);
  }

  return (
    <section class={styles.section} aria-labelledby="preferences-heading">
      <h2 id="preferences-heading" class={styles.sectionHeading}>
        <Settings2 size={18} aria-hidden="true" /> Preferences
      </h2>
      <p class={styles.sectionDescription}>
        Settings synced across all your devices.
      </p>

      <div class={formStyles.fieldGroup}>
        <div>
          <label htmlFor="pref-theme" class={formStyles.label}>Theme</label>
          <Select
            id="pref-theme"
            options={[
              { value: 'light', label: 'Light' },
              { value: 'dark', label: 'Dark' },
              { value: 'system', label: 'System' },
            ]}
            value={(preferences.theme as string) || 'system'}
            onChange={async (theme) => {
              document.documentElement.setAttribute('data-theme', theme === 'system'
                ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
                : theme);
              localStorage.setItem('icefall-theme', theme);
              await savePreference({ theme });
            }}
            fullWidth
          />
        </div>
        <div>
          <label htmlFor="pref-timezone" class={formStyles.label}>Timezone</label>
          <Select
            id="pref-timezone"
            options={Intl.supportedValuesOf('timeZone').map(tz => ({ value: tz, label: tz.replace(/_/g, ' ') }))}
            value={(preferences.timezone as string) || Intl.DateTimeFormat().resolvedOptions().timeZone}
            onChange={async (timezone) => {
              await savePreference({ timezone });
            }}
            fullWidth
          />
        </div>
        <div>
          <label class={formStyles.label}>
            <input
              type="checkbox"
              class={formStyles.checkbox}
              checked={preferences.email_notifications !== false}
              onChange={async (e) => {
                const email_notifications = (e.target as HTMLInputElement).checked;
                await savePreference({ email_notifications });
              }}
            />
            {' '}Email notifications for deploy events
          </label>
        </div>
      </div>
      {saving && <p class={styles.savingIndicator}>Saving...</p>}
    </section>
  );
}
