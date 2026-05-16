import { useEffect, useState, useCallback } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $serverMetricsRange } from '@stores/server';
import { api } from '@lib/api';
import { createVisibleInterval } from '@lib/visibility';
import type { ServerMetricsSnapshot } from '@lib/types';
import { formatPercent } from '@lib/format';
import MetricsChart from '@islands/shared/MetricsChart/MetricsChart';
import { Cpu, MemoryStick, HardDrive } from 'lucide-preact';
import styles from './metrics-detail-page.module.css';

type RangeKey = '5m' | '30m' | '1h' | '6h' | '24h' | '7d';

const RANGES: { key: RangeKey; label: string; minutes: number; useMemory: boolean }[] = [
  { key: '5m', label: '5 min', minutes: 5, useMemory: true },
  { key: '30m', label: '30 min', minutes: 30, useMemory: false },
  { key: '1h', label: '1 hour', minutes: 60, useMemory: false },
  { key: '6h', label: '6 hours', minutes: 360, useMemory: false },
  { key: '24h', label: '24 hours', minutes: 1440, useMemory: false },
  { key: '7d', label: '7 days', minutes: 10080, useMemory: false },
];

export default function MetricsDetailPage() {
  const cached = useStore($serverMetricsRange);
  const [data, setData] = useState<ServerMetricsSnapshot[]>(cached);
  const [range, setRange] = useState<RangeKey>('30m');
  const [loading, setLoading] = useState(false);

  const fetchData = useCallback(async (rangeKey: RangeKey) => {
    const cfg = RANGES.find(r => r.key === rangeKey)!;
    try {
      let result: ServerMetricsSnapshot[];
      if (cfg.useMemory) {
        const res = await api.getServerMetricsHistory(120);
        result = res.data;
      } else {
        const from = new Date(Date.now() - cfg.minutes * 60 * 1000).toISOString();
        const to = new Date().toISOString();
        const res = await api.getServerMetricsRange(from, to, 1000);
        result = res.data;
      }
      setData(result);
      $serverMetricsRange.set(result);
    } catch {}
  }, []);

  useEffect(() => {
    let active = true;
    const cfg = RANGES.find(r => r.key === range)!;
    const pollMs = cfg.useMemory ? 2000 : 30000;

    setLoading(true);
    fetchData(range).then(() => { if (active) setLoading(false); });

    const stopPolling = createVisibleInterval(() => fetchData(range), pollMs);
    return () => { active = false; stopPolling(); };
  }, [range, fetchData]);

  const cpuData = data.map(s => ({ timestamp: s.timestamp, value: s.cpu_percent }));
  const memData = data.map(s => ({
    timestamp: s.timestamp,
    value: s.memory_total_bytes > 0 ? (s.memory_used_bytes / s.memory_total_bytes) * 100 : 0,
  }));
  const diskData = data.map(s => ({
    timestamp: s.timestamp,
    value: s.disk_total_bytes > 0 ? (s.disk_used_bytes / s.disk_total_bytes) * 100 : 0,
  }));

  const latest = data.length > 0 ? data[data.length - 1] : null;
  const latestCpu = latest?.cpu_percent ?? 0;
  const latestMem = latest && latest.memory_total_bytes > 0
    ? (latest.memory_used_bytes / latest.memory_total_bytes) * 100 : 0;
  const latestDisk = latest && latest.disk_total_bytes > 0
    ? (latest.disk_used_bytes / latest.disk_total_bytes) * 100 : 0;

  return (
    <div>
      <nav aria-label="Breadcrumb" class={styles.breadcrumb}>
        <a href="/server">Server</a> / <span aria-current="page">Metrics</span>
      </nav>

      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Server Metrics</h1>
        <div class={styles.rangeToggle} role="group" aria-label="Time range">
          {RANGES.map(r => (
            <button
              key={r.key}
              type="button"
              class={`${styles.rangeButton} ${range === r.key ? styles.rangeActive : ''}`}
              onClick={() => setRange(r.key)}
              aria-pressed={range === r.key}
            >
              {r.label}
            </button>
          ))}
        </div>
      </div>

      <div class={styles.chartsGrid}>
        <div class={styles.chartCard}>
          <div class={styles.chartHeader}>
            <div class={styles.chartLabel}>
              <Cpu size={18} aria-hidden="true" />
              <span>CPU</span>
            </div>
            <span class={styles.chartValue}>{formatPercent(latestCpu)}</span>
          </div>
          <div class={styles.chartBody}>
            <MetricsChart
              data={cpuData}
              label="CPU"
              formatValue={v => formatPercent(v)}
              min={0}
              max={100}
              color="var(--color-primary)"
              height={200}
            />
          </div>
        </div>

        <div class={styles.chartCard}>
          <div class={styles.chartHeader}>
            <div class={styles.chartLabel}>
              <MemoryStick size={18} aria-hidden="true" />
              <span>Memory</span>
            </div>
            <span class={styles.chartValue}>{Math.round(latestMem)}%</span>
          </div>
          <div class={styles.chartBody}>
            <MetricsChart
              data={memData}
              label="Memory"
              formatValue={v => `${Math.round(v)}%`}
              min={0}
              max={100}
              color="var(--color-info)"
              height={200}
            />
          </div>
        </div>

        <div class={styles.chartCard}>
          <div class={styles.chartHeader}>
            <div class={styles.chartLabel}>
              <HardDrive size={18} aria-hidden="true" />
              <span>Disk</span>
            </div>
            <span class={styles.chartValue}>{Math.round(latestDisk)}%</span>
          </div>
          <div class={styles.chartBody}>
            <MetricsChart
              data={diskData}
              label="Disk"
              formatValue={v => `${Math.round(v)}%`}
              min={0}
              max={100}
              color="var(--color-warning)"
              height={200}
            />
          </div>
        </div>
      </div>

      <p class={styles.dataInfo} role="status" aria-live="polite">
        {data.length} data points
        {loading && data.length > 0 && ' · refreshing...'}
      </p>
    </div>
  );
}
