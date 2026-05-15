import { useEffect, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $serverStatus } from '@stores/server';
import { $servers, $serverCount } from '@stores/servers';
import { api } from '@lib/api';
import { createSSEClient } from '@lib/sse';
import type { Server, ServerMetricsSnapshot, ServerResources } from '@lib/types';
import { formatBytes, formatPercent } from '@lib/format';
import { createVisibleInterval } from '@lib/visibility';
import ProgressBar from '@islands/shared/ProgressBar/ProgressBar';
import Sparkline from '@islands/shared/Sparkline/Sparkline';
import ServerHealthStrip from '@islands/dashboard/ServerHealthStrip/ServerHealthStrip';
import styles from './server-stats.module.css';

function parseResources(raw: string | null): ServerResources | null {
  if (!raw) return null;
  try { return JSON.parse(raw); } catch { return null; }
}

function aggregateServers(servers: Server[]) {
  let totalCpuWeighted = 0;
  let totalCores = 0;
  let totalRamUsed = 0;
  let totalRamTotal = 0;
  let totalDiskUsed = 0;
  let totalDiskTotal = 0;

  for (const s of servers) {
    const r = parseResources(s.resources);
    if (!r) continue;
    const cores = r.cpu_cores || 1;
    totalCpuWeighted += r.cpu_percent * cores;
    totalCores += cores;
    totalRamUsed += r.ram_used_bytes;
    totalRamTotal += r.ram_total_bytes;
    totalDiskUsed += r.disk_used_bytes;
    totalDiskTotal += r.disk_total_bytes;
  }

  return {
    cpu_percent: totalCores > 0 ? totalCpuWeighted / totalCores : 0,
    memory_used_bytes: totalRamUsed,
    memory_total_bytes: totalRamTotal,
    disk_used_bytes: totalDiskUsed,
    disk_total_bytes: totalDiskTotal,
  };
}

export default function ServerStats() {
  const status = useStore($serverStatus);
  const servers = useStore($servers);
  const serverCount = useStore($serverCount);
  const [loaded, setLoaded] = useState(false);
  const [history, setHistory] = useState<ServerMetricsSnapshot[]>([]);

  useEffect(() => {
    let active = true;

    async function fetchAll() {
      try {
        const data = await api.getServerStatus();
        if (active) $serverStatus.set(data);
      } catch {}
      try {
        const { data } = await api.getServerMetricsHistory(60);
        if (active) setHistory(data);
      } catch {}

      try {
        const { data } = await api.listServers();
        if (active) {
          $servers.set(data);
          $serverCount.set(data.length);
        }
      } catch {}

      if (active) setLoaded(true);
    }

    fetchAll();
    const stopPolling = createVisibleInterval(fetchAll, 5_000);

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
      stopPolling();
      sse.close();
    };
  }, []);

  const isMultiServer = serverCount >= 2;
  const aggregate = isMultiServer ? aggregateServers(servers) : null;
  const displayStatus = aggregate || status;

  if (!displayStatus) {
    if (loaded) return null;
    return (
      <div class={styles.grid}>
        {[0, 1, 2].map((i) => (
          <div key={i} class={styles.skeleton} />
        ))}
      </div>
    );
  }

  const cpuData = history.map(s => s.cpu_percent);
  const memData = history.map(s => s.memory_total_bytes > 0 ? (s.memory_used_bytes / s.memory_total_bytes) * 100 : 0);
  const diskData = history.map(s => s.disk_total_bytes > 0 ? (s.disk_used_bytes / s.disk_total_bytes) * 100 : 0);

  return (
    <div>
      <ServerHealthStrip servers={servers} />
      <div class={styles.grid}>
        <div class={styles.card}>
          <ProgressBar
            label={isMultiServer ? 'CPU (avg)' : 'CPU'}
            value={displayStatus.cpu_percent}
            max={100}
            formatValue={(v) => formatPercent(v)}
          />
          {!isMultiServer && cpuData.length > 1 && (
            <div class={styles.sparklineWrap}>
              <Sparkline data={cpuData} max={100} color="var(--color-primary)" />
            </div>
          )}
        </div>
        <div class={styles.card}>
          <ProgressBar
            label={isMultiServer ? 'Memory (total)' : 'Memory'}
            value={displayStatus.memory_used_bytes}
            max={displayStatus.memory_total_bytes}
            formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`}
          />
          {!isMultiServer && memData.length > 1 && (
            <div class={styles.sparklineWrap}>
              <Sparkline data={memData} max={100} color="var(--color-info)" />
            </div>
          )}
        </div>
        <div class={styles.card}>
          <ProgressBar
            label={isMultiServer ? 'Disk (total)' : 'Disk'}
            value={displayStatus.disk_used_bytes}
            max={displayStatus.disk_total_bytes}
            formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`}
          />
          {!isMultiServer && diskData.length > 1 && (
            <div class={styles.sparklineWrap}>
              <Sparkline data={diskData} max={100} color="var(--color-warning)" />
            </div>
          )}
        </div>
        <a href={isMultiServer ? '/servers' : '/server/metrics'} class={styles.detailLink}>
          {isMultiServer ? 'View all servers' : 'View details'} &rarr;
        </a>
      </div>
    </div>
  );
}
