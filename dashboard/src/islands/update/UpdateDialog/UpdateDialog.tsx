import { useEffect, useRef, useState, useCallback } from 'preact/hooks';
import { createPortal } from 'preact/compat';
import { useStore } from '@nanostores/preact';
import {
  CheckCircle2,
  XCircle,
  Circle,
  Loader2,
  AlertTriangle,
  Info,
  ExternalLink,
  ArrowRight,
  ChevronDown,
  ChevronUp,
} from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import { $updateInfo, $updateStatus, $updateDialogOpen } from '@stores/update';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { UpdateStep, UpdateStatus } from '@stores/update';
import styles from './update-dialog.module.css';

const STEP_LABELS: Record<string, string> = {
  checking_compatibility: 'Checking compatibility',
  creating_backup: 'Creating backup',
  downloading: 'Downloading update',
  verifying_integrity: 'Verifying integrity',
  applying_migrations: 'Applying update',
  restarting: 'Restarting Icefall',
  verifying_health: 'Verifying health',
};

function formatDuration(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  const mins = Math.floor(secs / 60);
  const remaining = Math.round(secs % 60);
  return remaining > 0 ? `${mins}m ${remaining}s` : `${mins}m`;
}

function StepIcon({ step }: { step: UpdateStep }) {
  switch (step.status) {
    case 'done':
      return <CheckCircle2 size={18} aria-hidden="true" class={`${styles.stepIcon} ${styles.stepIconDone}`} />;
    case 'running':
      return <Loader2 size={18} aria-hidden="true" class={`${styles.stepIcon} ${styles.stepIconRunning} ${styles.spinner}`} />;
    case 'failed':
      return <XCircle size={18} aria-hidden="true" class={`${styles.stepIcon} ${styles.stepIconFailed}`} />;
    default:
      return <Circle size={18} aria-hidden="true" class={`${styles.stepIcon} ${styles.stepIconPending}`} />;
  }
}

export default function UpdateDialog() {
  const info = useStore($updateInfo);
  const status = useStore($updateStatus);
  const open = useStore($updateDialogOpen);
  const [starting, setStarting] = useState(false);
  const [errorExpanded, setErrorExpanded] = useState(false);
  const [elapsedTimers, setElapsedTimers] = useState<Record<string, number>>({});

  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelWrapRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const isUpdating = status?.state === 'downloading' || status?.state === 'applying';
  const isCompleted = status?.state === 'completed';
  const isFailed = status?.state === 'failed';

  // Focus management
  useEffect(() => {
    if (open) {
      previousFocusRef.current = document.activeElement as HTMLElement;
      requestAnimationFrame(() => {
        const btn = cancelWrapRef.current?.querySelector('button');
        btn?.focus();
      });
    } else if (previousFocusRef.current) {
      previousFocusRef.current.focus();
      previousFocusRef.current = null;
    }
  }, [open]);

  // Body scroll lock
  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [open]);

  // Escape key (only when not updating)
  useEffect(() => {
    if (!open || isUpdating) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        handleClose();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, isUpdating, isCompleted, isFailed]);

  // Focus trap
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
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
    },
    []
  );

  // Poll update status while active
  useEffect(() => {
    if (!isUpdating) {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
      return;
    }

    async function poll() {
      try {
        const res = await api.getUpdateStatus();
        $updateStatus.set(res.data);
      } catch {
        // SSE drop during restart is expected
      }
    }

    pollRef.current = setInterval(poll, 2000);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [isUpdating]);

  // Track elapsed time for running steps
  useEffect(() => {
    if (!isUpdating) return;

    const interval = setInterval(() => {
      setElapsedTimers((prev) => {
        const next = { ...prev };
        if (status?.steps) {
          for (const step of status.steps) {
            if (step.status === 'running') {
              next[step.name] = (next[step.name] ?? 0) + 1;
            }
          }
        }
        return next;
      });
    }, 1000);

    return () => clearInterval(interval);
  }, [isUpdating, status?.steps]);

  function handleClose() {
    $updateDialogOpen.set(false);
    if (isCompleted || isFailed) {
      $updateStatus.set(null);
      setElapsedTimers({});
      setErrorExpanded(false);
    }
  }

  async function handleBeginUpdate() {
    setStarting(true);
    try {
      const res = await api.applyUpdate();
      $updateStatus.set(res.data);
    } catch (err: any) {
      addToast('error', err.message || 'Failed to start update');
    }
    setStarting(false);
  }

  function getTotalDuration(): string {
    if (!status?.steps) return '';
    const total = status.steps.reduce((sum, s) => sum + (s.duration_secs ?? 0), 0);
    return formatDuration(total);
  }

  if (!open || !info) return null;

  const titleId = 'update-dialog-title';

  // Determine which view to render
  const showPreUpdate = !isUpdating && !isCompleted && !isFailed;
  const failedStep = status?.steps?.find((s) => s.status === 'failed');

  // Portal to body: the sidebar (where this island mounts) has a transform,
  // which would otherwise trap our position:fixed backdrop inside the sidebar.
  return createPortal(
    <div
      class={styles.backdrop}
      onClick={isUpdating ? undefined : handleClose}
    >
      <div
        ref={dialogRef}
        class={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {/* Pre-update state */}
        {showPreUpdate && (
          <>
            <h2 id={titleId} class={styles.title}>Update Icefall</h2>

            <div class={styles.versionTransition}>
              <span>v{info.current_version}</span>
              <ArrowRight size={14} aria-hidden="true" class={styles.versionArrow} />
              <span>v{info.latest_version}</span>
            </div>

            {info.changelog_highlights.length > 0 && (
              <ul class={styles.changelogList}>
                {info.changelog_highlights.slice(0, 5).map((highlight, i) => (
                  <li key={i} class={styles.changelogItem}>{highlight}</li>
                ))}
              </ul>
            )}

            {info.changelog_url && (
              <a
                href={info.changelog_url}
                target="_blank"
                rel="noopener noreferrer"
                class={styles.releaseLink}
              >
                View full release notes
                <ExternalLink size={14} aria-hidden="true" />
              </a>
            )}

            {info.breaking && info.breaking_changes && (
              <div class={styles.breakingCallout} role="alert">
                <AlertTriangle size={18} aria-hidden="true" class={styles.breakingIcon} />
                <div>
                  <strong>Breaking changes</strong>
                  <br />
                  {info.breaking_changes}
                </div>
              </div>
            )}

            <div class={styles.infoBox}>
              <Info size={18} aria-hidden="true" class={styles.infoIcon} />
              A backup will be created before updating.
            </div>

            <div class={styles.actions}>
              <div ref={cancelWrapRef} style={{ display: 'contents' }}>
                <Button variant="ghost" onClick={handleClose}>
                  Cancel
                </Button>
              </div>
              <Button variant="primary" onClick={handleBeginUpdate} loading={starting}>
                Begin update
              </Button>
            </div>
          </>
        )}

        {/* Active update state */}
        {isUpdating && (
          <>
            <h2 id={titleId} class={styles.title}>
              Updating to v{status?.target_version || info.latest_version}
            </h2>

            <div class={styles.stepsList} role="list" aria-label="Update progress">
              {(status?.steps ?? []).map((step) => (
                <div key={step.name} class={styles.step} role="listitem">
                  <StepIcon step={step} />
                  {step.name === 'downloading' && step.status === 'running' && step.progress != null ? (
                    <div class={styles.progressWrap}>
                      <div class={styles.progressLabel}>
                        <span class={styles.stepLabel}>{step.label || STEP_LABELS[step.name]}</span>
                        <span class={styles.progressPercent}>{Math.round(step.progress)}%</span>
                      </div>
                      <div
                        class={styles.progressTrack}
                        role="progressbar"
                        aria-valuenow={Math.round(step.progress)}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-label={`Download progress: ${Math.round(step.progress)}%`}
                      >
                        <div
                          class={styles.progressFill}
                          style={{ transform: `scaleX(${step.progress / 100})` }}
                        />
                      </div>
                    </div>
                  ) : (
                    <span class={`${styles.stepLabel} ${step.status === 'pending' ? styles.stepLabelPending : ''}`}>
                      {step.label || STEP_LABELS[step.name]}
                    </span>
                  )}
                  {step.status === 'done' && step.duration_secs != null && (
                    <span class={styles.stepDuration}>{formatDuration(step.duration_secs)}</span>
                  )}
                  {step.status === 'running' && elapsedTimers[step.name] != null && step.name !== 'downloading' && (
                    <span class={styles.stepDuration}>{formatDuration(elapsedTimers[step.name])}</span>
                  )}
                </div>
              ))}
            </div>
          </>
        )}

        {/* Success state */}
        {isCompleted && (
          <>
            <div class={styles.successCenter}>
              <CheckCircle2 size={48} aria-hidden="true" class={styles.successIcon} />
              <h2 id={titleId} class={styles.successHeading}>Update complete</h2>
              <p class={styles.successSub}>
                Now running v{status?.target_version || info.latest_version}.
                {' '}Total time: {getTotalDuration()}.
              </p>
              <Button variant="primary" onClick={handleClose}>Close</Button>
            </div>
          </>
        )}

        {/* Error state */}
        {isFailed && (
          <>
            <h2 id={titleId} class={styles.title}>Update failed</h2>

            <div class={styles.stepsList} role="list" aria-label="Update progress">
              {(status?.steps ?? []).map((step) => (
                <div key={step.name} class={styles.step} role="listitem">
                  <StepIcon step={step} />
                  <span class={`${styles.stepLabel} ${step.status === 'pending' ? styles.stepLabelPending : ''}`}>
                    {step.label || STEP_LABELS[step.name]}
                  </span>
                  {step.status === 'done' && step.duration_secs != null && (
                    <span class={styles.stepDuration}>{formatDuration(step.duration_secs)}</span>
                  )}
                </div>
              ))}
            </div>

            {failedStep?.error && (
              <p class={styles.errorMessage}>{failedStep.error}</p>
            )}

            {status?.error && (
              <>
                <button
                  type="button"
                  class={styles.errorDetailsToggle}
                  onClick={() => setErrorExpanded(!errorExpanded)}
                  aria-expanded={errorExpanded}
                >
                  {errorExpanded ? (
                    <>
                      <ChevronUp size={12} aria-hidden="true" />
                      Hide error details
                    </>
                  ) : (
                    <>
                      <ChevronDown size={12} aria-hidden="true" />
                      View error details
                    </>
                  )}
                </button>
                {errorExpanded && (
                  <div class={styles.errorDetails} role="log">
                    {status.error}
                  </div>
                )}
              </>
            )}

            <div class={styles.actions}>
              <Button variant="secondary" onClick={handleClose}>Close</Button>
            </div>
          </>
        )}
      </div>
    </div>,
    document.body,
  );
}
