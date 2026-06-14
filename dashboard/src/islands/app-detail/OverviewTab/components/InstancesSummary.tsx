import { useEffect, useState } from 'preact/hooks';
import type { AppInstance } from '@lib/types';
import { api } from '@lib/api';
import styles from './instances-summary.module.css';

type Props = {
  appId: string;
  /** Desired instance count from the app record. */
  desiredInstances: number;
};

/**
 * Compact instance health summary for the app overview. Renders nothing for
 * single-instance apps (desiredInstances <= 1).
 */
export default function InstancesSummary({ appId, desiredInstances }: Props) {
  const [instances, setInstances] = useState<AppInstance[] | null>(null);

  useEffect(() => {
    if (desiredInstances <= 1) return;
    let active = true;
    api
      .listInstances(appId)
      .then(({ data }) => {
        if (active) setInstances(data);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [appId, desiredInstances]);

  if (desiredInstances <= 1 || instances === null) return null;

  const healthy = instances.filter((i) => i.status === 'running').length;
  const total = instances.length;
  const allHealthy = total > 0 && healthy === total;

  return (
    <div class={styles.panel}>
      <h3 class={styles.sectionTitle}>Instances</h3>
      <div class={styles.summaryRow}>
        <span
          class={`${styles.count} ${allHealthy ? styles.countHealthy : styles.countDegraded}`}
        >
          {healthy}/{total}
        </span>
        <span class={styles.label}>instances healthy</span>
      </div>
      <ul class={styles.dots} aria-label="Instance health">
        {instances.map((instance) => (
          <li
            key={instance.id}
            class={`${styles.dot} ${
              instance.status === 'running'
                ? styles.dotHealthy
                : instance.status === 'failed' || instance.status === 'unhealthy'
                  ? styles.dotFailed
                  : styles.dotPending
            }`}
            title={`${instance.server_id}: ${instance.status}`}
          />
        ))}
      </ul>
      <a class={styles.link} href={`/apps/${appId}/instances`}>
        View all instances
      </a>
    </div>
  );
}
