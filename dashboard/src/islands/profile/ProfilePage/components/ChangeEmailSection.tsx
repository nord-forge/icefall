import { useState } from 'preact/hooks';
import { Mail } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import styles from '../profile-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  onChangeEmail: (newEmail: string, password: string) => Promise<string>;
};

export default function ChangeEmailSection({ onChangeEmail }: Props) {
  const [newEmail, setNewEmail] = useState('');
  const [emailPassword, setEmailPassword] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [msg, setMsg] = useState('');
  const [err, setErr] = useState('');

  async function handleSubmit() {
    setErr('');
    setMsg('');

    const trimmed = newEmail.trim();
    if (!trimmed || !trimmed.includes('@')) {
      setErr('Enter a valid email address');
      return;
    }

    setSubmitting(true);
    try {
      const message = await onChangeEmail(trimmed, emailPassword);
      setMsg(message);
      setNewEmail('');
      setEmailPassword('');
    } catch (e: any) {
      setErr(e.message || 'Failed to update email');
    }
    setSubmitting(false);
  }

  return (
    <section class={styles.section} aria-labelledby="email-heading">
      <h2 id="email-heading" class={styles.sectionHeading}>
        <Mail size={18} aria-hidden="true" /> Change Email
      </h2>
      <p class={styles.sectionDescription}>
        Enter your password to confirm the email change.
      </p>

      {err && <p class={styles.feedbackError} role="alert">{err}</p>}
      {msg && <p class={styles.feedbackSuccess} role="status">{msg}</p>}

      <div class={formStyles.fieldGroup}>
        <Input
          label="New email address"
          name="new-email"
          id="new-email"
          type="email"
          value={newEmail}
          onChange={setNewEmail}
          placeholder="new@example.com"
        />
        <Input
          label="Current password"
          name="email-confirm-password"
          id="email-confirm-password"
          type="password"
          value={emailPassword}
          onChange={setEmailPassword}
        />
      </div>
      <div class={styles.formActions}>
        <Button
          variant="primary"
          onClick={handleSubmit}
          loading={submitting}
          disabled={!newEmail.trim() || !emailPassword}
        >
          Update Email
        </Button>
      </div>
    </section>
  );
}
