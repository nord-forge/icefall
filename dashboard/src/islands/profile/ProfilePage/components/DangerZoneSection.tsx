import { useState } from 'preact/hooks';
import { AlertTriangle } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import styles from '../profile-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  onDeleteAccount: (password: string) => Promise<void>;
};

export default function DangerZoneSection({ onDeleteAccount }: Props) {
  const [deletePassword, setDeletePassword] = useState('');
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [err, setErr] = useState('');

  async function handleDeleteAccount() {
    setErr('');
    setSubmitting(true);
    try {
      await onDeleteAccount(deletePassword);
    } catch (e: any) {
      setErr(e.message || 'Failed to delete account');
    }
    setSubmitting(false);
  }

  return (
    <section class={styles.section} aria-labelledby="danger-heading">
      <h2 id="danger-heading" class={styles.sectionHeadingDanger}>
        <AlertTriangle size={18} aria-hidden="true" /> Danger Zone
      </h2>
      <p class={styles.sectionDescription}>
        Permanently delete your account and all associated data. This action cannot be undone.
      </p>

      {err && <p class={styles.feedbackError} role="alert">{err}</p>}

      {!deleteConfirm ? (
        <Button variant="danger" onClick={() => setDeleteConfirm(true)}>
          Delete Account
        </Button>
      ) : (
        <div class={styles.dangerCard}>
          <p class={styles.dangerText}>
            Enter your password to confirm account deletion. All your sessions, tokens, and data will be permanently removed.
          </p>
          <div class={formStyles.fieldGroup}>
            <Input
              label="Password"
              name="delete-password"
              id="delete-password"
              type="password"
              value={deletePassword}
              onChange={setDeletePassword}
            />
          </div>
          <div class={styles.formActions}>
            <Button variant="ghost" onClick={() => { setDeleteConfirm(false); setDeletePassword(''); setErr(''); }}>
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={handleDeleteAccount}
              loading={submitting}
              disabled={!deletePassword}
            >
              Permanently Delete Account
            </Button>
          </div>
        </div>
      )}
    </section>
  );
}
