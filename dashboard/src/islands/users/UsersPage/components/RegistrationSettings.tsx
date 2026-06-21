import type { RegistrationSettings as RegistrationSettingsType } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import Toggle from '@islands/shared/Toggle/Toggle';
import styles from '../users-page.module.css';

const ROLE_OPTIONS = [
  { value: 'admin', label: 'Admin' },
  { value: 'deployer', label: 'Deployer' },
  { value: 'viewer', label: 'Viewer' },
];

type Props = {
  settings: RegistrationSettingsType;
  domainsInput: string;
  loading: boolean;
  saving: boolean;
  onSettingsChange: (settings: RegistrationSettingsType) => void;
  onDomainsChange: (value: string) => void;
  onSave: () => void;
};

export default function RegistrationSettings({
  settings,
  domainsInput,
  loading,
  saving,
  onSettingsChange,
  onDomainsChange,
  onSave,
}: Props) {
  return (
    <section class={styles.section}>
      <div class={styles.sectionHeader}>
        <h2 class={styles.sectionTitle}>Registration Settings</h2>
      </div>

      {loading ? (
        <p class={styles.loadingText}>Loading settings...</p>
      ) : (
        <div class={`${styles.card} ${styles.cardCompact}`}>
          <div class={styles.regGrid}>
            <div class={styles.regRow}>
              <label htmlFor="allow-registration" class={styles.regLabel}>
                Allow public registration
              </label>
              <Toggle
                id="allow-registration"
                label="Allow public registration"
                hideLabel
                checked={settings.allow_registration}
                onChange={(v) =>
                  onSettingsChange({ ...settings, allow_registration: v })
                }
              />
            </div>

            {settings.allow_registration && (
              <div class={styles.regRow}>
                <Input
                  label="Allowed domains"
                  name="allowed-domains"
                  id="allowed-domains"
                  value={domainsInput}
                  onChange={onDomainsChange}
                  placeholder="company.com, example.org"
                  className={styles.regInput}
                />
              </div>
            )}

            <div class={styles.regRow}>
              <label htmlFor="default-role" class={styles.regLabel}>
                Default role
              </label>
              <Select
                id="default-role"
                options={ROLE_OPTIONS}
                value={settings.default_role}
                onChange={(role) =>
                  onSettingsChange({
                    ...settings,
                    default_role: role,
                  })
                }
                size="sm"
              />
            </div>
          </div>

          <div class={styles.cardActions}>
            <Button
              variant="primary"
              onClick={onSave}
              loading={saving}
              size="sm"
            >
              Save Settings
            </Button>
          </div>
        </div>
      )}
    </section>
  );
}
