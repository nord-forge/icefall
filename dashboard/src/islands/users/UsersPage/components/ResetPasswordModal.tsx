import { useEffect, useRef } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import { Copy } from 'lucide-preact';
import styles from '../users-page.module.css';

type Props = {
  userId: string;
  email: string;
  tempPassword?: string;
  resetting: boolean;
  onConfirm: () => void;
  onClose: () => void;
};

export default function ResetPasswordModal({
  email,
  tempPassword,
  resetting,
  onConfirm,
  onClose,
}: Props) {
  const dialogRef = useRef<HTMLDivElement>(null);

  // a11y [WCAG 2.4.3]: focus trap for password reset modal
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const focusable = dialog.querySelectorAll<HTMLElement>(
      'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    function handleTab(e: KeyboardEvent) {
      if (e.key !== 'Tab') return;
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last?.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first?.focus(); }
    }
    dialog.addEventListener('keydown', handleTab);
    first?.focus();
    return () => dialog.removeEventListener('keydown', handleTab);
  }, []);

  return (
    <div ref={dialogRef} class={styles.overlay} role="dialog" aria-modal="true" aria-label="Reset password">
      <div class={styles.modal}>
        <h3 class={styles.modalTitle}>Reset Password</h3>
        {tempPassword ? (
          <>
            <p class={styles.modalText}>
              A temporary password has been generated for <strong>{email}</strong>.
              Share it securely -- it will not be shown again.
            </p>
            <div class={styles.tokenRow}>
              <code class={styles.tokenValue}>{tempPassword}</code>
              <button
                type="button"
                class={styles.iconButton}
                onClick={() => navigator.clipboard.writeText(tempPassword)}
                aria-label="Copy temporary password"
              >
                <Copy size={14} aria-hidden="true" />
              </button>
            </div>
            <div class={styles.cardActions}>
              <Button variant="ghost" onClick={onClose}>
                Close
              </Button>
            </div>
          </>
        ) : (
          <>
            <p class={styles.modalText}>
              This will generate a new temporary password for <strong>{email}</strong> and
              invalidate all their existing sessions.
            </p>
            <div class={styles.cardActions}>
              <Button variant="ghost" onClick={onClose}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={onConfirm}
                loading={resetting}
              >
                Generate Temporary Password
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
