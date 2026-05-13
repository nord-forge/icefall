import { useState } from 'preact/hooks';
import { Lock } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from '../profile-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  onChangePassword: (currentPassword: string, newPassword: string) => Promise<string>;
  onPasswordChanged: () => void;
};

export default function ChangePasswordSection({ onChangePassword, onPasswordChanged }: Props) {
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [msg, setMsg] = useState('');
  const [err, setErr] = useState('');

  async function handleSubmit() {
    setErr('');
    setMsg('');

    if (newPassword.length < 12) {
      setErr('New password must be at least 12 characters');
      return;
    }
    if (newPassword !== confirmPassword) {
      setErr('Passwords do not match');
      return;
    }

    setSubmitting(true);
    try {
      const message = await onChangePassword(currentPassword, newPassword);
      setMsg(message);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      onPasswordChanged();
    } catch (e: any) {
      setErr(e.message || 'Failed to change password');
    }
    setSubmitting(false);
  }

  return (
    <section class={styles.section} aria-labelledby="password-heading">
      <h2 id="password-heading" class={styles.sectionHeading}>
        <Lock size={18} aria-hidden="true" /> Change Password
      </h2>
      <p class={styles.sectionDescription}>
        After changing your password, all other sessions will be signed out.
      </p>

      {err && <p class={styles.feedbackError} role="alert">{err}</p>}
      {msg && <p class={styles.feedbackSuccess} role="status">{msg}</p>}

      <div class={formStyles.fieldGroup}>
        <div>
          {/* a11y [1.3.1]: label associated with input */}
          <label htmlFor="current-password" class={formStyles.label}>Current password</label>
          <input
            id="current-password"
            class={formStyles.input}
            type="password"
            autoComplete="current-password"
            value={currentPassword}
            onInput={e => setCurrentPassword((e.target as HTMLInputElement).value)}
          />
        </div>
        <div>
          <label htmlFor="new-password" class={formStyles.label}>New password</label>
          <input
            id="new-password"
            class={formStyles.input}
            type="password"
            autoComplete="new-password"
            value={newPassword}
            onInput={e => setNewPassword((e.target as HTMLInputElement).value)}
          />
          <p class={formStyles.hint}>Minimum 12 characters</p>
        </div>
        <div>
          <label htmlFor="confirm-password" class={formStyles.label}>Confirm new password</label>
          <input
            id="confirm-password"
            class={formStyles.input}
            type="password"
            autoComplete="new-password"
            value={confirmPassword}
            onInput={e => setConfirmPassword((e.target as HTMLInputElement).value)}
            onKeyDown={e => { if (e.key === 'Enter') handleSubmit(); }}
          />
        </div>
      </div>
      <div class={styles.formActions}>
        <Button
          variant="primary"
          onClick={handleSubmit}
          loading={submitting}
          disabled={!currentPassword || !newPassword || !confirmPassword}
        >
          Update Password
        </Button>
      </div>
    </section>
  );
}
