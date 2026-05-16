import { useEffect, useState } from 'preact/hooks';
import type { ServerAppInstance, AppInstanceStatus } from '@lib/types';
import { api } from '@lib/api';
import Badge from '@islands/shared/Badge/Badge';
import styles from './instances-section.module.css';

type Props = {
  serverId: string;
};

const STATUS_VARIANT: Record<AppInstanceStatus, 'success' | 'warning' | 'error' | 'info' | 'default'> = {
  running: 'success',
  deploying: 'info',
  unhealthy: 'warning',
  failed: 'error',
  stopped: 'default',
};

/** App instances running on a server, grouped by app. */
export default function InstancesSection({ serverId }: Props) {
  const [instances, setInstances] = useState<ServerAppInstance[] | null>(null);

  useEffect(() => {
    let active = true;
    api
      .listServerInstances(serverId)
      .then(({ data }) => {
        if (active) setInstances(data);
      })
      .catch(() => {
        if (active) setInstances([]);
      });
    return () => {
      active = false;
    };
  }, [serverId]);

  if (instances === null) return null;

  return (
    <div class={styles.section}>
      <h2 class={styles.sectionTitle}>
        App instances
        <span class={styles.count}>{instances.length}</span>
      </h2>

      {instances.length === 0 ? (
        <p class={styles.empty}>No app instances are running on this server.</p>
      ) : (
        <ul class={styles.appList}>
          {groupByApp(instances).map(({ appId, appName, items }) => (
            <li key={appId} class={styles.appGroup}>
              <a class={styles.appName} href={`/apps/${appId}/instances`}>
                {appName}
              </a>
              <span class={styles.appCount}>
                {items.length} instance{items.length === 1 ? '' : 's'}
              </span>
              <ul class={styles.instanceList}>
                {items.map((inst) => (
                  <li key={inst.id} class={styles.instanceRow}>
                    <Badge
                      label={inst.status}
                      variant={STATUS_VARIANT[inst.status] ?? 'default'}
                    />
                    <span class={styles.port}>
                      {inst.host_port ? `:${inst.host_port}` : '—'}
                    </span>
                  </li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

type AppGroup = {
  appId: string;
  appName: string;
  items: ServerAppInstance[];
};

function groupByApp(instances: ServerAppInstance[]): AppGroup[] {
  const groups = new Map<string, AppGroup>();
  for (const inst of instances) {
    let group = groups.get(inst.app_id);
    if (!group) {
      group = { appId: inst.app_id, appName: inst.app_name, items: [] };
      groups.set(inst.app_id, group);
    }
    group.items.push(inst);
  }
  return [...groups.values()];
}
