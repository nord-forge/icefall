import { useEffect } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $offlineServers, addOfflineServer, removeOfflineServer } from '@stores/offline-servers';
import { createSSEClient } from '@lib/sse';
import { AlertTriangle } from 'lucide-preact';
import styles from './offline-server-banner.module.css';

export default function OfflineServerBanner() {
  const offlineServers = useStore($offlineServers);

  useEffect(() => {
    const sse = createSSEClient('/api/v1/events', {
      'server.disconnected': (data: any) => {
        if (data.server_id && data.server_name) {
          addOfflineServer(data.server_id, data.server_name);
        }
      },
      'server.connected': (data: any) => {
        if (data.server_id) {
          removeOfflineServer(data.server_id);
        }
      },
    });
    return () => sse.close();
  }, []);

  if (offlineServers.length === 0) return null;

  const message =
    offlineServers.length === 1
      ? `${offlineServers[0].name} is offline`
      : `${offlineServers.length} servers are offline`;

  return (
    <div class={styles.banner} role="alert">
      <AlertTriangle size={16} aria-hidden="true" />
      <span class={styles.text}>
        {message}
        {offlineServers.length === 1 && (
          <a href={`/servers/${offlineServers[0].id}`} class={styles.link}>
            View server
          </a>
        )}
      </span>
    </div>
  );
}
