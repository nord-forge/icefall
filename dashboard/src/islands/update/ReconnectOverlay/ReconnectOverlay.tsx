import { useEffect, useRef, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import Button from '@islands/shared/Button/Button';
import { $updateStatus } from '@stores/update';
import { addToast } from '@stores/toast';
import { request } from '@lib/api';
import styles from './reconnect-overlay.module.css';

const POLL_INTERVAL = 2000;
const SLOW_THRESHOLD = 30_000;
const TIMEOUT_THRESHOLD = 60_000;

export default function ReconnectOverlay() {
  const status = useStore($updateStatus);
  const [visible, setVisible] = useState(false);
  const [exiting, setExiting] = useState(false);
  const [elapsedMs, setElapsedMs] = useState(0);
  const [retrying, setRetrying] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const startRef = useRef(0);

  // Detect the restarting step to show overlay
  const restartingStep = status?.steps?.find(
    (s) => s.name === 'restarting' && s.status === 'running'
  );

  useEffect(() => {
    if (restartingStep) {
      setVisible(true);
      setExiting(false);
      setElapsedMs(0);
      startRef.current = Date.now();

      // Timer for elapsed
      timerRef.current = setInterval(() => {
        setElapsedMs(Date.now() - startRef.current);
      }, 1000);

      // Polling for reconnection
      pollRef.current = setInterval(async () => {
        try {
          // Cache-busting query keeps each poll a genuine reachability probe.
          await request(`/server/status?t=${Date.now()}`);
          handleReconnected();
        } catch {
          // Still down
        }
      }, POLL_INTERVAL);
    }

    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [restartingStep?.status]);

  function handleReconnected() {
    if (pollRef.current) clearInterval(pollRef.current);
    if (timerRef.current) clearInterval(timerRef.current);
    setExiting(true);
    addToast('success', 'Reconnected to Icefall');
    setTimeout(() => {
      setVisible(false);
      setExiting(false);
    }, 300);
  }

  async function handleRetry() {
    setRetrying(true);
    try {
      await request(`/server/status?t=${Date.now()}`);
      handleReconnected();
    } catch {
      // Still down
    }
    setRetrying(false);
  }

  if (!visible) return null;

  const isSlow = elapsedMs >= SLOW_THRESHOLD && elapsedMs < TIMEOUT_THRESHOLD;
  const isTimedOut = elapsedMs >= TIMEOUT_THRESHOLD;

  return (
    <div
      class={`${styles.overlay} ${exiting ? styles.overlayExit : ''}`}
      role="alert"
      aria-live="assertive"
    >
      <div class={styles.content}>
        <div class={styles.heading}>
          Reconnecting to Icefall
          <span class={styles.dots} aria-hidden="true">
            <span class={styles.dot} />
            <span class={styles.dot} />
            <span class={styles.dot} />
          </span>
        </div>
        <p class={styles.subtext}>
          {isTimedOut
            ? 'Unable to reach the server.'
            : isSlow
              ? 'Taking longer than expected.'
              : 'This usually takes about 10 seconds.'}
        </p>
        {isTimedOut && (
          <div class={styles.retryWrap}>
            <Button variant="primary" onClick={handleRetry} loading={retrying}>
              Retry now
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
