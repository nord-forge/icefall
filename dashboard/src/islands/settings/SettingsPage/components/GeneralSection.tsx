import { useState, useEffect } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $settings } from '@stores/settings';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import { Save, Globe } from 'lucide-preact';
import { TIMEZONES } from '@lib/timezones';
import { api } from '@lib/api';
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
    api.getSettings().then(d => {
      $settings.set(d.data);
      if (d.data.base_domain) setBaseDomain(d.data.base_domain);
    }).catch(() => {});
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      await api.updateBaseDomain(baseDomain);
      onSaveMessage('Domain saved');
    } catch { onSaveMessage('Save failed'); }
    setSaving(false);
  }

  return (
    <div class={styles.section}>
      <h2 class={styles.sectionHeading}><Globe size={18} aria-hidden="true" /> General</h2>
      <div class={formStyles.fieldRow}>
        <Input
          label="Platform Name"
          name="sp-platform-name"
          id="sp-platform-name"
          value={platformName}
          onChange={setPlatformName}
          placeholder="Icefall"
          helpText="Displayed in the dashboard header and emails."
        />
        <Input
          label="Base Domain"
          name="sp-base-domain"
          id="sp-base-domain"
          value={baseDomain}
          onChange={setBaseDomain}
          placeholder="apps.example.com"
          helpText="Used for app subdomains and SSL certificates."
        />
      </div>
      <div class={formStyles.fieldRow}>
        <Input
          label="Recovery Email"
          name="sp-recovery-email"
          id="sp-recovery-email"
          type="email"
          value={recoveryEmail}
          onChange={setRecoveryEmail}
          placeholder="recovery@example.com"
          helpText="Receives password reset links if the admin account is locked out."
        />
        <div>
          <Select
            id="sp-timezone"
            options={TIMEZONES.map(tz => ({ value: tz, label: tz.replace(/_/g, ' ') }))}
            value={timezone}
            onChange={setTimezone}
            fullWidth
          />
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
