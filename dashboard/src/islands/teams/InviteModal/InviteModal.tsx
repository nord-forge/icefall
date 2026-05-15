import { useState, useEffect, useRef, useCallback } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import styles from './invite-modal.module.css';

type Props = {
  teamId: string;
  open: boolean;
  onClose: () => void;
  onInvited: () => void;
};

const roleOptions = [
  { value: 'admin', label: 'Admin' },
  { value: 'member', label: 'Member' },
  { value: 'viewer', label: 'Viewer' },
];

export default function InviteModal({ teamId, open, onClose, onInvited }: Props) {
  const [email, setEmail] = useState('');
  const [role, setRole] = useState('member');
  const [sending, setSending] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  // Focus management
  useEffect(() => {
    if (open) {
      previousFocusRef.current = document.activeElement as HTMLElement;
      requestAnimationFrame(() => {
        const firstInput = dialogRef.current?.querySelector('input');
        firstInput?.focus();
      });
    } else if (previousFocusRef.current) {
      previousFocusRef.current.focus();
      previousFocusRef.current = null;
    }
  }, [open]);

  // Lock body scroll
  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [open]);

  // Escape key
  useEffect(() => {
    if (!open) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  // Focus trap
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key !== 'Tab' || !dialogRef.current) return;

    const focusable = dialogRef.current.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
    );
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey) {
      if (document.activeElement === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }, []);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    const trimmedEmail = email.trim();
    if (!trimmedEmail) return;

    setSending(true);
    try {
      await api.inviteTeamMember(teamId, trimmedEmail, role);
      addToast('success', `Invitation sent to ${trimmedEmail}.`);
      setEmail('');
      setRole('member');
      onInvited();
      onClose();
    } catch {
      addToast('error', 'Failed to send invitation. Please try again.');
    } finally {
      setSending(false);
    }
  }

  if (!open) return null;

  const titleId = 'invite-modal-title';

  return (
    <div class={styles.backdrop} onClick={onClose}>
      {/* a11y [WCAG 4.1.2]: dialog role with aria-modal and labelling */}
      <div
        ref={dialogRef}
        class={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <h2 id={titleId} class={styles.title}>Invite team member</h2>
        <form onSubmit={handleSubmit} class={styles.form}>
          <Input
            label="Email address"
            name="invite-email"
            type="email"
            value={email}
            placeholder="colleague@example.com"
            required
            onChange={setEmail}
          />
          <div>
            <label class={styles.fieldLabel} htmlFor="invite-role-select">Role</label>
            <Select
              id="invite-role-select"
              options={roleOptions}
              value={role}
              onChange={setRole}
              aria-label="Member role"
            />
          </div>
          <div class={styles.actions}>
            <Button variant="secondary" onClick={onClose} disabled={sending}>
              Cancel
            </Button>
            <Button variant="primary" type="submit" loading={sending} disabled={!email.trim()}>
              Send invitation
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
