import type { Server, ServerResources } from '@lib/types';
import { formatBytes, formatPercent } from '@lib/format';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import ProgressBar from '@islands/shared/ProgressBar/ProgressBar';
import styles from './server-card.module.css';

type Props = {
  server: Server;
}

function parseResources(raw: string | null): ServerResources | null {
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export default function ServerCard({ server }: Props) {
  const resources = parseResources(server.resources);
  const isControlPlane = server.role === 'control-plane';

  return (
    <div class={styles.card}>
      <a href={`/servers/${server.id}`} class={styles.cardLink}>
        <div class={styles.header}>
          <span class={styles.name}>{server.name}</span>
          <StatusDot status={server.status} />
        </div>

        <span class={styles.host}>{server.host}</span>

        <div class={styles.badges}>
          {isControlPlane && (
            <span class={styles.badgeControlPlane}>Control plane</span>
          )}
          {!isControlPlane && (
            <span class={styles.badgeWorker}>Worker</span>
          )}
          {typeof server.app_count === 'number' && (
            <span class={styles.badgeApps}>
              {server.app_count} {server.app_count === 1 ? 'app' : 'apps'}
            </span>
          )}
        </div>

        {resources && (
          <div class={styles.metrics}>
            <ProgressBar
              label="CPU"
              value={resources.cpu_percent}
              max={100}
              formatValue={(v) => formatPercent(v)}
            />
            <ProgressBar
              label="RAM"
              value={resources.ram_used_bytes}
              max={resources.ram_total_bytes}
              formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`}
            />
          </div>
        )}

        {!resources && server.status === 'online' && (
          <p class={styles.noMetrics}>Waiting for metrics...</p>
        )}
      </a>
    </div>
  );
}
