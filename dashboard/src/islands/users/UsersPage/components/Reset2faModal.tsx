import { useEffect, useRef } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import styles from '../users-page.module.css';

type Props = {
  email: string;
  resetting: boolean;
  onConfirm: () => void;
  onClose: () => void;
};

export default function Reset2faModal({
  email,
  resetting,
  onConfirm,
  onClose,
}: Props) {
  const dialogRef = useRef<HTMLDivElement>(null);

  // a11y [WCAG 2.4.3]: focus trap for 2FA reset modal
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
    <div ref={dialogRef} class={styles.overlay} role="dialog" aria-modal="true" aria-label="Reset two-factor authentication">
      <div class={styles.modal}>
        <h3 class={styles.modalTitle}>Reset 2FA</h3>
        <p class={styles.modalText}>
          This will disable two-factor authentication for <strong>{email}</strong> and
          invalidate all their sessions. They will need to set up 2FA again on their next login.
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
            Reset 2FA
          </Button>
        </div>
      </div>
    </div>
  );
}
