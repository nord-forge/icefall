import type { Server } from '@lib/types';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import styles from './server-health-strip.module.css';

type Props = {
  servers: Server[];
};

export default function ServerHealthStrip({ servers }: Props) {
  if (servers.length < 2) return null;

  return (
    <div class={styles.strip} role="list" aria-label="Server status">
      {servers.map((server) => (
        <a
          key={server.id}
          href={`/servers/${server.id}`}
          class={styles.item}
          role="listitem"
        >
          <StatusDot status={server.status} showLabel={false} />
          <span class={styles.name}>{server.name}</span>
        </a>
      ))}
    </div>
  );
}
