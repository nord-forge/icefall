import { useEffect, useState } from 'preact/hooks';
import type { AppInstance, AppInstanceStatus, Server } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { formatRelativeTime, shortSha } from '@lib/format';
import { RefreshCw, Trash2 } from 'lucide-preact';
import Badge from '@islands/shared/Badge/Badge';
import Button from '@islands/shared/Button/Button';
import Table from '@islands/shared/Table/Table';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import EmptyState from '@islands/shared/EmptyState/EmptyState';
import styles from './instances-tab.module.css';

type Props = {
  appId: string;
};

const STATUS_VARIANT: Record<AppInstanceStatus, 'success' | 'warning' | 'error' | 'info' | 'default'> = {
  running: 'success',
  deploying: 'info',
  unhealthy: 'warning',
  failed: 'error',
  stopped: 'default',
};

const COLUMNS = [
  { key: 'server', header: 'Server' },
  { key: 'status', header: 'Status' },
  { key: 'container', header: 'Container' },
  { key: 'port', header: 'Port' },
  { key: 'started', header: 'Started' },
  { key: 'actions', header: 'Actions', srOnly: true },
];

export default function InstancesTab({ appId }: Props) {
  const [instances, setInstances] = useState<AppInstance[]>([]);
  const [servers, setServers] = useState<Server[]>([]);
  const [loading, setLoading] = useState(true);
  const [removeId, setRemoveId] = useState<string | null>(null);
  const [removing, setRemoving] = useState(false);

  async function refresh() {
    try {
      const [inst, srv] = await Promise.all([
        api.listInstances(appId),
        api.listServers(),
      ]);
      setInstances(inst.data);
      setServers(srv.data);
    } catch (err: any) {
      addToast('error', err.message || 'Failed to load instances');
    }
    setLoading(false);
  }

  useEffect(() => {
    refresh();
    // Instances change as deploys/health checks run; poll periodically.
    const timer = window.setInterval(refresh, 10000);
    return () => window.clearInterval(timer);
  }, [appId]);

  function serverName(serverId: string): string {
    return servers.find((s) => s.id === serverId)?.name || serverId;
  }

  async function handleRemove() {
    if (!removeId) return;
    setRemoving(true);
    try {
      await api.deleteInstance(appId, removeId);
      addToast('success', 'Instance removed');
      setRemoveId(null);
      await refresh();
    } catch (err: any) {
      addToast('error', err.message || 'Failed to remove instance');
    }
    setRemoving(false);
  }

  if (loading) {
    return <div class={styles.loading}>Loading instances...</div>;
  }

  const healthy = instances.filter((i) => i.status === 'running').length;

  if (instances.length === 0) {
    return (
      <div class={styles.container}>
        <EmptyState
          title="No instances"
          description="This app runs as a single instance. Increase the desired instance count in Settings → Scaling to run multiple."
        />
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <div class={styles.summary}>
        <span class={styles.summaryCount}>
          {healthy}/{instances.length}
        </span>
        <span class={styles.summaryLabel}>instances healthy</span>
      </div>

      <Table columns={COLUMNS}>
        {instances.map((instance) => (
          <tr key={instance.id} class={styles.row}>
            <td class={styles.cell}>{serverName(instance.server_id)}</td>
            <td class={styles.cell}>
              <Badge
                label={instance.status}
                variant={STATUS_VARIANT[instance.status] ?? 'default'}
              />
            </td>
            <td class={`${styles.cell} ${styles.mono}`}>
              {instance.container_id ? shortSha(instance.container_id) : '—'}
            </td>
            <td class={`${styles.cell} ${styles.mono}`}>
              {instance.host_port ?? '—'}
            </td>
            <td class={styles.cell}>{formatRelativeTime(instance.created_at)}</td>
            <td class={`${styles.cell} ${styles.actions}`}>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setRemoveId(instance.id)}
                aria-label={`Remove instance on ${serverName(instance.server_id)}`}
              >
                <Trash2 size={14} aria-hidden="true" /> Remove
              </Button>
            </td>
          </tr>
        ))}
      </Table>

      <div class={styles.refreshRow}>
        <Button variant="ghost" size="sm" onClick={refresh}>
          <RefreshCw size={14} aria-hidden="true" /> Refresh
        </Button>
      </div>

      <ConfirmDialog
        open={removeId !== null}
        title="Remove instance?"
        description="This stops and removes the instance container. The reverse proxy stops routing traffic to it. If the app is still above its desired count, the instance stays gone; otherwise the health monitor may start a replacement."
        confirmLabel="Remove instance"
        variant="danger"
        loading={removing}
        onConfirm={handleRemove}
        onCancel={() => setRemoveId(null)}
      />
    </div>
  );
}
