import { useEffect, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $servers, $serversLoading, $serverCount } from '@stores/servers';
import { api } from '@lib/api';
import { createSSEClient } from '@lib/sse';
import type { Server } from '@lib/types';
import ServerCard from '@islands/servers/ServerCard/ServerCard';
import AddServerPanel from '@islands/servers/AddServerPanel/AddServerPanel';
import Button from '@islands/shared/Button/Button';
import { Plus, Server as ServerIcon } from 'lucide-preact';
import styles from './servers-page.module.css';

export default function ServersPage() {
  const servers = useStore($servers);
  const loading = useStore($serversLoading);
  const [showAddPanel, setShowAddPanel] = useState(false);

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        const { data } = await api.listServers();
        if (!active) return;

        const cp = data.find((s) => s.role === 'control-plane');
        if (cp && !cp.resources) {
          try {
            const status = await api.getServerStatus();
            cp.resources = JSON.stringify({
              cpu_percent: status.cpu_percent,
              cpu_cores: 1,
              ram_used_bytes: status.memory_used_bytes,
              ram_total_bytes: status.memory_total_bytes,
              disk_used_bytes: status.disk_used_bytes,
              disk_total_bytes: status.disk_total_bytes,
              load_average: [],
            });
          } catch {}
        }

        $servers.set(data);
        $serverCount.set(data.length);
      } catch {}
      if (active) $serversLoading.set(false);
    }

    load();

    const sse = createSSEClient('/api/v1/events', {
      'server.connected': (data: any) => {
        $servers.set(
          $servers.get().map((s) =>
            s.id === data.server_id ? { ...s, status: 'online' as const } : s
          )
        );
      },
      'server.disconnected': (data: any) => {
        $servers.set(
          $servers.get().map((s) =>
            s.id === data.server_id ? { ...s, status: 'offline' as const } : s
          )
        );
      },
    });

    return () => {
      active = false;
      sse.close();
    };
  }, []);

  function handleServerAdded(server: Server) {
    $servers.set([...$servers.get(), server]);
    $serverCount.set($servers.get().length);
  }

  if (loading) {
    return (
      <div>
        <div class={styles.pageHeader}>
          <h1 class={styles.pageTitle}>Servers</h1>
        </div>
        <div class={styles.grid}>
          {[0, 1, 2].map((i) => (
            <div key={i} class={styles.skeleton} />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Servers</h1>
        <Button variant="primary" onClick={() => setShowAddPanel(true)}>
          <Plus size={14} /> Add server
        </Button>
      </div>

      {showAddPanel && (
        <AddServerPanel
          onClose={() => setShowAddPanel(false)}
          onServerAdded={handleServerAdded}
        />
      )}

      {servers.length === 0 && !showAddPanel && (
        <div class={styles.empty}>
          <ServerIcon size={32} aria-hidden="true" />
          <p class={styles.emptyTitle}>No servers registered</p>
          <p class={styles.emptyDescription}>
            Add a server to get started.
          </p>
          <Button variant="primary" onClick={() => setShowAddPanel(true)}>
            <Plus size={14} /> Add your first server
          </Button>
        </div>
      )}

      <div class={styles.grid}>
        {servers.map((server) => (
          <ServerCard key={server.id} server={server} />
        ))}
      </div>
    </div>
  );
}
