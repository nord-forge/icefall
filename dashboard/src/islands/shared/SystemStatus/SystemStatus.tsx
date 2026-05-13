import { useEffect, useState, useRef } from 'preact/hooks';
import { api } from '@lib/api';
import { createSSEClient } from '@lib/sse';
import { formatBytes, formatPercent } from '@lib/format';
import type { ServerStatus } from '@lib/types';
import styles from './system-status.module.css';

type Health = 'healthy' | 'degraded' | 'down' | 'loading';

export default function SystemStatus() {
  const [health, setHealth] = useState<Health>('loading');
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [showTooltip, setShowTooltip] = useState(false);
  const [appCount, setAppCount] = useState(0);
  const hideTimeout = useRef<number>();

  useEffect(() => {
    let active = true;

    async function check() {
      try {
        const data = await api.getServerStatus();
        if (!active) return;
        setStatus(data);

        if (data.cpu_percent > 95 || (data.memory_used_bytes / data.memory_total_bytes) > 0.95) {
          setHealth('degraded');
        } else {
          setHealth('healthy');
        }
      } catch {
        if (active) setHealth('down');
      }

      try {
        const { data } = await api.listApps();
        if (active) setAppCount(data.length);
      } catch {}
    }

    check();
    const interval = setInterval(check, 30_000);

    const sse = createSSEClient('/api/v1/events', {
      'server.status': () => { if (active) check(); },
      'server.connected': () => { if (active) check(); },
      'server.disconnected': () => { if (active) check(); },
      'deploy.status': () => { if (active) check(); },
      'metrics.update': (data: any) => {
        if (!active || !data) return;
        setStatus((prev) => prev ? {
          ...prev,
          cpu_percent: data.cpu_percent ?? prev.cpu_percent,
          memory_used_bytes: data.memory_used_bytes ?? prev.memory_used_bytes,
          memory_total_bytes: data.memory_total_bytes ?? prev.memory_total_bytes,
          disk_used_bytes: data.disk_used_bytes ?? prev.disk_used_bytes,
          disk_total_bytes: data.disk_total_bytes ?? prev.disk_total_bytes,
        } : prev);
      },
    });

    return () => { active = false; clearInterval(interval); sse.close(); };
  }, []);

  const dotClass = {
    healthy: styles.dotHealthy,
    degraded: styles.dotDegraded,
    down: styles.dotDown,
    loading: styles.dotLoading,
  }[health];

  const label = {
    healthy: 'Operational',
    degraded: 'Degraded',
    down: 'Unreachable',
    loading: 'Checking...',
  }[health];

  function handleEnter() {
    clearTimeout(hideTimeout.current);
    setShowTooltip(true);
  }

  function handleLeave() {
    hideTimeout.current = window.setTimeout(() => setShowTooltip(false), 150);
  }

  return (
    <div
      class={styles.wrapper}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
      onFocus={handleEnter}
      onBlur={handleLeave}
    >
      <button
        type="button"
        class={`${styles.dot} ${dotClass}`}
        aria-label={`System status: ${label}`}
        tabIndex={0}
      />

      {showTooltip && (
        <div class={styles.tooltip} role="status">
          <div class={styles.tooltipHeader}>
            <span class={`${styles.tooltipDot} ${dotClass}`} aria-hidden="true" />
            <span class={styles.tooltipLabel}>{label}</span>
          </div>

          {status && (
            <dl class={styles.tooltipGrid}>
              <dt>CPU</dt>
              <dd>{formatPercent(status.cpu_percent)}</dd>
              <dt>Memory</dt>
              <dd>{formatBytes(status.memory_used_bytes)} / {formatBytes(status.memory_total_bytes)}</dd>
              <dt>Disk</dt>
              <dd>{formatBytes(status.disk_used_bytes)} / {formatBytes(status.disk_total_bytes)}</dd>
              <dt>Apps</dt>
              <dd>{appCount}</dd>
            </dl>
          )}

          {health === 'down' && (
            <p class={styles.tooltipHint}>Cannot reach the Icefall daemon.</p>
          )}
        </div>
      )}
    </div>
  );
}
