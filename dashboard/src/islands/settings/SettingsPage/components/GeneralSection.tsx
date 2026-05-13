import { useState, useEffect } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $settings } from '@stores/settings';
import Button from '@islands/shared/Button/Button';
import Select from '@islands/shared/Select/Select';
import { Save, Globe } from 'lucide-preact';
import { TIMEZONES } from '@lib/timezones';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  onSaveMessage: (msg: string) => void;
};

export default function GeneralSection({ onSaveMessage }: Props) {
  const cachedSettings = useStore($settings);
  const [baseDomain, setBaseDomain] = useState(cachedSettings?.base_domain || '');
  const [platformName, setPlatformName] = useState('');
  const [recoveryEmail, setRecoveryEmail] = useState('');
  const [timezone, setTimezone] = useState(() => {
    if (typeof globalThis !== 'undefined' && typeof Intl !== 'undefined') {
      try { return Intl.DateTimeFormat().resolvedOptions().timeZone; } catch {}
    }
    return 'UTC';
  });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetch('/api/v1/settings', { credentials: 'same-origin' }).then(r => r.json()).then(d => {
      $settings.set(d.data);
      if (d.data.base_domain) setBaseDomain(d.data.base_domain);
    }).catch(() => {});
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      await fetch('/api/v1/settings/base-domain', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ base_domain: baseDomain }),
      });
      onSaveMessage('Domain saved');
    } catch { onSaveMessage('Save failed'); }
    setSaving(false);
  }

  return (
    <div class={styles.section}>
      <h2 class={styles.sectionHeading}><Globe size={18} aria-hidden="true" /> General</h2>
      <div class={formStyles.fieldRow}>
        <div>
          <label htmlFor="sp-platform-name" class={formStyles.label}>Platform Name</label>
          <input id="sp-platform-name" class={formStyles.input} value={platformName} onInput={e => setPlatformName((e.target as HTMLInputElement).value)} placeholder="Icefall" />
          <p class={formStyles.hint}>Displayed in the dashboard header and emails.</p>
        </div>
        <div>
          <label htmlFor="sp-base-domain" class={formStyles.label}>Base Domain</label>
          <input id="sp-base-domain" class={formStyles.input} value={baseDomain} onInput={e => setBaseDomain((e.target as HTMLInputElement).value)} placeholder="apps.example.com" />
          <p class={formStyles.hint}>Used for app subdomains and SSL certificates.</p>
        </div>
      </div>
      <div class={formStyles.fieldRow}>
        <div>
          <label htmlFor="sp-recovery-email" class={formStyles.label}>Recovery Email</label>
          <input id="sp-recovery-email" class={formStyles.input} type="email" autoComplete="email" value={recoveryEmail} onInput={e => setRecoveryEmail((e.target as HTMLInputElement).value)} placeholder="recovery@example.com" />
          <p class={formStyles.hint}>Receives password reset links if the admin account is locked out.</p>
        </div>
        <div>
          <label htmlFor="sp-timezone" class={formStyles.label}>Timezone</label>
          <Select
            id="sp-timezone"
            options={TIMEZONES.map(tz => ({ value: tz, label: tz.replace(/_/g, ' ') }))}
            value={timezone}
            onChange={setTimezone}
            fullWidth
          />
          <p class={formStyles.hint}>Used for log timestamps, backup schedules, and notifications.</p>
        </div>
      </div>
      <div class={styles.saveRow}>
        <Button variant="primary" onClick={handleSave} loading={saving}>
          <Save size={14} aria-hidden="true" /> Save General
        </Button>
      </div>
    </div>
  );
}
