import type { RegistrationSettings as RegistrationSettingsType } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import Select from '@islands/shared/Select/Select';
import styles from '../users-page.module.css';
import formStyles from '@styles/form.module.css';

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
              <button
                id="allow-registration"
                type="button"
                role="switch"
                aria-checked={settings.allow_registration}
                class={`${styles.toggle} ${settings.allow_registration ? styles.toggleOn : ''}`}
                onClick={() =>
                  onSettingsChange({
                    ...settings,
                    allow_registration: !settings.allow_registration,
                  })
                }
              >
                <span class={styles.toggleThumb}>
                  <svg class={styles.toggleIcon} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
                    <path class={styles.toggleCheck} d="M2.5 5 L4.5 7 L7.5 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
                    <path class={styles.toggleCross} d="M3 3 L7 7 M7 3 L3 7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
                  </svg>
                </span>
              </button>
            </div>

            {settings.allow_registration && (
              <div class={styles.regRow}>
                <label htmlFor="allowed-domains" class={styles.regLabel}>
                  Allowed domains
                </label>
                <input
                  id="allowed-domains"
                  class={`${formStyles.input} ${styles.regInput}`}
                  type="text"
                  value={domainsInput}
                  onInput={e =>
                    onDomainsChange((e.target as HTMLInputElement).value)
                  }
                  placeholder="company.com, example.org"
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
