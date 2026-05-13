import type { Server, ServerResources } from '@lib/types';
import { formatBytes, formatPercent } from '@lib/format';
import ProgressBar from '@islands/shared/ProgressBar/ProgressBar';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import styles from './server-select-step.module.css';

type Props = {
  servers: Server[];
  selectedId: string | null;
  onSelect: (id: string) => void;
};

function parseResources(raw: string | null): ServerResources | null {
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export default function ServerSelectStep({ servers, selectedId, onSelect }: Props) {
  return (
    <div>
      <p class={styles.description}>
        Choose which server should run this app. The recommended server has the most available resources.
      </p>
      {/* a11y [WCAG 4.1.2]: radiogroup with keyboard navigation */}
      <div class={styles.grid} role="radiogroup" aria-label="Select a server">
        {servers.map((server) => {
          const resources = parseResources(server.resources);
          const isSelected = selectedId === server.id;
          const isDisabled = server.status !== 'online';
          const isControlPlane = server.role === 'control-plane';

          return (
            <button
              key={server.id}
              type="button"
              role="radio"
              aria-checked={isSelected}
              aria-disabled={isDisabled}
              class={`${styles.card} ${isSelected ? styles.cardSelected : ''} ${isDisabled ? styles.cardDisabled : ''}`}
              onClick={() => !isDisabled && onSelect(server.id)}
              tabIndex={isSelected ? 0 : -1}
            >
              <div class={styles.cardHeader}>
                <span class={styles.name}>{server.name}</span>
                <div class={styles.cardBadges}>
                  {server.recommended && (
                    <span class={styles.recommendedBadge}>Recommended</span>
                  )}
                  {isDisabled && <span class={styles.offlineBadge}>Offline</span>}
                </div>
              </div>

              <div class={styles.cardMeta}>
                <span class={styles.host}>{server.host}</span>
                <span class={isControlPlane ? styles.roleCp : styles.roleWorker}>
                  {isControlPlane ? 'Control plane' : 'Worker'}
                </span>
              </div>

              {resources && (
                <div class={styles.metrics}>
                  <ProgressBar label="CPU" value={resources.cpu_percent} max={100} formatValue={(v) => formatPercent(v)} />
                  <ProgressBar label="RAM" value={resources.ram_used_bytes} max={resources.ram_total_bytes} formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`} />
                  <ProgressBar label="Disk" value={resources.disk_used_bytes} max={resources.disk_total_bytes} formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`} />
                </div>
              )}

              {typeof server.app_count === 'number' && (
                <span class={styles.appCount}>
                  {server.app_count} {server.app_count === 1 ? 'app' : 'apps'} deployed
                </span>
              )}

              {/* a11y [WCAG 1.4.1]: checkmark shape distinguishes selected state beyond color */}
              <div class={styles.radioIndicator} aria-hidden="true">
                {isSelected && (
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                    <path d="M2.5 6 L5 8.5 L9.5 3.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                  </svg>
                )}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
