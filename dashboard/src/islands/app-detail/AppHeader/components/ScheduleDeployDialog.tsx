import { useEffect, useRef, useState, useCallback } from 'preact/hooks';
import { createPortal } from 'preact/compat';
import Button from '@islands/shared/Button/Button';
import { CalendarClock } from 'lucide-preact';
import { currentTimeZone, loadTimeZone, wallTimeToUtcIso, utcIsoToWallTime } from '@lib/timezone';
import styles from './schedule-deploy-dialog.module.css';

type Props = {
  open: boolean;
  /** Heading verb — "Schedule" for a new deploy, "Reschedule" when editing one. */
  mode?: 'schedule' | 'reschedule';
  loading?: boolean;
  /** Pre-fill with this UTC ISO timestamp (e.g. the current scheduled time). */
  initialIso?: string;
  /** Receives an ISO 8601 (UTC) timestamp. */
  onConfirm: (isoUtc: string) => void;
  onCancel: () => void;
};

export default function ScheduleDeployDialog({
  open,
  mode = 'schedule',
  loading = false,
  initialIso,
  onConfirm,
  onCancel,
}: Props) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);
  const [tz, setTz] = useState(currentTimeZone());
  const [value, setValue] = useState('');
  const [error, setError] = useState('');

  // Refresh to the user's configured timezone (may differ from the browser's).
  useEffect(() => {
    loadTimeZone().then(setTz).catch(() => {});
  }, []);

  // The picker can't select a time already in the past (in the user's tz).
  const minLocal = utcIsoToWallTime(new Date(Date.now() + 60_000).toISOString(), tz);

  useEffect(() => {
    if (!open) return;
    const startIso = initialIso ?? new Date(Date.now() + 60 * 60_000).toISOString();
    setValue(utcIsoToWallTime(startIso, tz));
    setError('');
  }, [open, initialIso, tz]);

  useEffect(() => {
    if (open) {
      previousFocusRef.current = document.activeElement as HTMLElement;
      requestAnimationFrame(() => {
        dialogRef.current?.querySelector('input')?.focus();
      });
    } else if (previousFocusRef.current) {
      previousFocusRef.current.focus();
      previousFocusRef.current = null;
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        onCancel();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onCancel]);

  const handleConfirm = useCallback(() => {
    if (!value) {
      setError('Pick a date and time.');
      return;
    }
    const isoUtc = wallTimeToUtcIso(value, tz);
    if (Number.isNaN(new Date(isoUtc).getTime())) {
      setError('That date is not valid.');
      return;
    }
    if (new Date(isoUtc).getTime() <= Date.now()) {
      setError('Pick a time in the future.');
      return;
    }
    onConfirm(isoUtc);
  }, [value, tz, onConfirm]);

  if (!open) return null;

  const titleId = 'schedule-deploy-title';
  const descId = 'schedule-deploy-desc';
  const fieldId = 'schedule-deploy-when';
  const errId = 'schedule-deploy-error';
  const heading = mode === 'reschedule' ? 'Reschedule deploy' : 'Schedule deploy';

  return createPortal(
    <div class={styles.backdrop} onClick={onCancel}>
      {/* a11y [WCAG 4.1.2]: modal dialog with labelling and described error */}
      <div
        ref={dialogRef}
        class={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descId}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id={titleId} class={styles.title}>
          <CalendarClock size={18} aria-hidden="true" /> {heading}
        </h2>
        <p id={descId} class={styles.description}>
          The deploy runs automatically at the chosen time, in your timezone (<strong>{tz}</strong>).
        </p>

        {/* a11y [WCAG 3.3.2]: explicit label associated with the field */}
        <label class={styles.label} for={fieldId}>
          Deploy at
        </label>
        <input
          id={fieldId}
          class={styles.input}
          type="datetime-local"
          value={value}
          min={minLocal}
          aria-invalid={error ? 'true' : undefined}
          aria-describedby={error ? errId : undefined}
          onInput={(e) => {
            setValue((e.target as HTMLInputElement).value);
            setError('');
          }}
        />
        {error && (
          <p id={errId} class={styles.error} role="alert">
            {error}
          </p>
        )}

        <div class={styles.actions}>
          <Button variant="secondary" onClick={onCancel} disabled={loading}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleConfirm} loading={loading}>
            {mode === 'reschedule' ? 'Reschedule' : 'Schedule'}
          </Button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
