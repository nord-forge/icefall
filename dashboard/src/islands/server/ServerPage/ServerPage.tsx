import { useEffect, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $serverStatus, $serverMetricsHistory } from '@stores/server';
import { api } from '@lib/api';
import type { ServerStatus, ServerMetricsSnapshot } from '@lib/types';
import { formatBytes, formatPercent } from '@lib/format';
import ProgressBar from '@islands/shared/ProgressBar/ProgressBar';
import MetricsChart from '@islands/shared/MetricsChart/MetricsChart';
import Button from '@islands/shared/Button/Button';
import { Cpu, HardDrive, MemoryStick, Globe, Server, Hash, LayoutGrid, List, Clock } from 'lucide-preact';
import Skeleton from '@islands/shared/Skeleton/Skeleton';
import styles from './server-page.module.css';

type InfoItem = {
  icon: any;
  label: string;
  value: string;
  mono?: boolean;
}

type Range = '10m' | '1h';

export default function ServerPage() {
  const cachedStatus = useStore($serverStatus);
  const cachedHistory = useStore($serverMetricsHistory);
  const [status, setStatus] = useState<ServerStatus | null>(cachedStatus);
  const [serverIp, setServerIp] = useState('');
  const [loading, setLoading] = useState(!cachedStatus);
  const [viewMode, setViewMode] = useState<'cards' | 'table'>('cards');
  const [history, setHistory] = useState<ServerMetricsSnapshot[]>(cachedHistory);
  const [range, setRange] = useState<Range>('10m');

  const historyLimit = range === '10m' ? 60 : 360;

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        const data = await api.getServerStatus();
        if (active) { setStatus(data); $serverStatus.set(data); }
      } catch {}
      try {
        const { ip } = await api.getServerIp();
        if (active) setServerIp(ip);
      } catch {}
      try {
        const { data } = await api.getServerMetricsHistory(historyLimit);
        if (active) { setHistory(data); $serverMetricsHistory.set(data); }
      } catch {}
      if (active) setLoading(false);
    }

    load();
    const interval = setInterval(async () => {
      try {
        const [statusRes, historyRes] = await Promise.all([
          api.getServerStatus(),
          api.getServerMetricsHistory(historyLimit),
        ]);
        if (active) {
          setStatus(statusRes); $serverStatus.set(statusRes);
          setHistory(historyRes.data); $serverMetricsHistory.set(historyRes.data);
        }
      } catch {}
    }, 5_000);

    return () => { active = false; clearInterval(interval); };
  }, [historyLimit]);

  const cpuHistory = status ? history.map(s => ({ timestamp: s.timestamp, value: s.cpu_percent })) : [];
  const memHistory = status ? history.map(s => ({
    timestamp: s.timestamp,
    value: s.memory_total_bytes > 0 ? (s.memory_used_bytes / s.memory_total_bytes) * 100 : 0,
  })) : [];
  const diskHistory = status ? history.map(s => ({
    timestamp: s.timestamp,
    value: s.disk_total_bytes > 0 ? (s.disk_used_bytes / s.disk_total_bytes) * 100 : 0,
  })) : [];

  const infoItems: InfoItem[] = status ? [
    { icon: Hash, label: 'Version', value: status.version },
    { icon: Server, label: 'Status', value: status.status },
    ...(serverIp ? [{ icon: Globe, label: 'Server IP', value: serverIp, mono: true }] : []),
    { icon: MemoryStick, label: 'Total Memory', value: formatBytes(status.memory_total_bytes) },
    { icon: HardDrive, label: 'Total Disk', value: formatBytes(status.disk_total_bytes) },
    { icon: Cpu, label: 'CPU Usage', value: formatPercent(status.cpu_percent) },
  ] : [];

  return (
    <div>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Server</h1>
        <div class={styles.headerActions}>
          <div class={styles.rangeToggle}>
            <button
              type="button"
              class={`${styles.rangeButton} ${range === '10m' ? styles.rangeActive : ''}`}
              onClick={() => setRange('10m')}
              aria-pressed={range === '10m'}
            >
              <Clock size={12} aria-hidden="true" /> 10 min
            </button>
            <button
              type="button"
              class={`${styles.rangeButton} ${range === '1h' ? styles.rangeActive : ''}`}
              onClick={() => setRange('1h')}
              aria-pressed={range === '1h'}
            >
              <Clock size={12} aria-hidden="true" /> 1 hour
            </button>
          </div>
          <a href="/server/metrics" class={styles.detailsLink}>
            <Button variant="secondary" size="sm">Details</Button>
          </a>
        </div>
      </div>

      {loading && !status && (
        <div class={styles.metricsGrid}>
          {[0, 1, 2].map(i => (
            <div key={i} class={styles.metricCard}>
              <Skeleton width="40%" height="1rem" />
              <Skeleton width="100%" height="0.5rem" variant="rect" />
              <Skeleton width="100%" height="120px" variant="rect" />
            </div>
          ))}
        </div>
      )}

      {!loading && !status && (
        <div class={styles.emptyState}>
          <p class={styles.emptyTitle}>Unable to reach server</p>
          <p class={styles.emptyHint}>Make sure the Icefall daemon is running. Check <a href="/servers" data-astro-prefetch="hover">server status</a> for details.</p>
        </div>
      )}

      {status && (
      <div class={styles.metricsGrid}>
        <div class={styles.metricCard}>
          <div class={styles.metricHeader}>
            <div class={styles.metricIcon}><Cpu size={20} aria-hidden="true" /></div>
            <div class={styles.metricContent}>
              <ProgressBar
                label="CPU"
                value={status.cpu_percent}
                max={100}
                formatValue={(v) => formatPercent(v)}
              />
            </div>
          </div>
          <div class={styles.chartWrap}>
            <MetricsChart
              data={cpuHistory}
              label="CPU"
              formatValue={v => formatPercent(v)}
              min={0}
              max={100}
              color="var(--color-primary)"
            />
          </div>
        </div>

        <div class={styles.metricCard}>
          <div class={styles.metricHeader}>
            <div class={styles.metricIcon}><MemoryStick size={20} aria-hidden="true" /></div>
            <div class={styles.metricContent}>
              <ProgressBar
                label="Memory"
                value={status.memory_used_bytes}
                max={status.memory_total_bytes}
                formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`}
              />
            </div>
          </div>
          <div class={styles.chartWrap}>
            <MetricsChart
              data={memHistory}
              label="Memory"
              formatValue={v => `${Math.round(v)}%`}
              min={0}
              max={100}
              color="var(--color-info)"
            />
          </div>
        </div>

        <div class={styles.metricCard}>
          <div class={styles.metricHeader}>
            <div class={styles.metricIcon}><HardDrive size={20} aria-hidden="true" /></div>
            <div class={styles.metricContent}>
              <ProgressBar
                label="Disk"
                value={status.disk_used_bytes}
                max={status.disk_total_bytes}
                formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`}
              />
            </div>
          </div>
          <div class={styles.chartWrap}>
            <MetricsChart
              data={diskHistory}
              label="Disk"
              formatValue={v => `${Math.round(v)}%`}
              min={0}
              max={100}
              color="var(--color-warning)"
            />
          </div>
        </div>
      </div>
      )}

      {status && (<>
      <div class={styles.sectionHeader}>
        <h2 class={styles.sectionTitle}>System Info</h2>
        <div class={styles.viewToggle}>
          <button
            type="button"
            class={`${styles.toggleButton} ${viewMode === 'cards' ? styles.toggleActive : ''}`}
            onClick={() => setViewMode('cards')}
            aria-pressed={viewMode === 'cards'}
            aria-label="Card view"
          >
            <LayoutGrid size={14} aria-hidden="true" />
          </button>
          <button
            type="button"
            class={`${styles.toggleButton} ${viewMode === 'table' ? styles.toggleActive : ''}`}
            onClick={() => setViewMode('table')}
            aria-pressed={viewMode === 'table'}
            aria-label="Table view"
          >
            <List size={14} aria-hidden="true" />
          </button>
        </div>
      </div>

      {viewMode === 'cards' ? (
        <div class={styles.infoGrid}>
          {infoItems.map(item => (
            <div key={item.label} class={styles.infoCard}>
              <div class={styles.infoCardIcon}>
                <item.icon size={18} aria-hidden="true" />
              </div>
              <div class={styles.infoCardContent}>
                <span class={styles.infoCardLabel}>{item.label}</span>
                <span class={item.mono ? styles.infoCardValueMono : styles.infoCardValue}>{item.value}</span>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div class={styles.detailCard}>
          <dl class={styles.detailList}>
            {infoItems.map(item => (
              <div key={item.label} class={styles.detailRow}>
                <dt class={styles.detailLabel}>{item.label}</dt>
                <dd class={item.mono ? styles.detailValueMono : styles.detailValue}>{item.value}</dd>
              </div>
            ))}
          </dl>
        </div>
      )}
      </>)}
    </div>
  );
}
