import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import styles from './uptime-timeline.module.css';

type HealthEvent = {
  status: string;
  checked_at: string;
}

type Props = {
  appId: string;
}

const RANGES = [
  { key: '24h', label: '24h', hours: 24 },
  { key: '7d', label: '7d', hours: 168 },
  { key: '30d', label: '30d', hours: 720 },
];

const SEGMENTS = 48;

const SEGMENT_STYLES: Record<string, string> = {
  healthy: styles.segmentHealthy,
  unhealthy: styles.segmentUnhealthy,
  none: styles.segmentNone,
};

export default function UptimeTimeline({ appId }: Props) {
  const [events, setEvents] = useState<HealthEvent[]>([]);
  const [range, setRange] = useState('24h');
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);

  useEffect(() => {
    api.getHealthEvents(appId, 1000)
      .then((data) => {
        const allEvents = data.data?.flatMap((c) => c.recent_events || []) || [];
        setEvents(allEvents);
      })
      .catch(() => {});
  }, [appId]);

  const selectedRange = RANGES.find((r) => r.key === range) || RANGES[0];
  const now = Date.now();
  const rangeMs = selectedRange.hours * 60 * 60 * 1000;
  const segmentMs = rangeMs / SEGMENTS;

  const segments = Array.from({ length: SEGMENTS }, (_, i) => {
    const segStart = now - rangeMs + i * segmentMs;
    const segEnd = segStart + segmentMs;
    const segEvents = events.filter((e) => {
      const t = new Date(e.checked_at).getTime();
      return t >= segStart && t < segEnd;
    });
    if (segEvents.length === 0) return 'none';
    return segEvents.some((e) => e.status === 'unhealthy') ? 'unhealthy' : 'healthy';
  });

  const rangeStart = now - rangeMs;
  const rangeEvents = events.filter((e) => new Date(e.checked_at).getTime() >= rangeStart);
  const healthyCount = rangeEvents.filter((e) => e.status === 'healthy').length;
  const uptimePercent = rangeEvents.length > 0
    ? ((healthyCount / rangeEvents.length) * 100).toFixed(1)
    : '100.0';

  return (
    <div class={styles.wrapper}>
      <div class={styles.header}>
        <span class={styles.title}>Uptime</span>
        <div class={styles.headerRight}>
          <span class={`${styles.uptimeValue} ${parseFloat(uptimePercent) >= 99 ? styles.uptimeGood : styles.uptimeWarn}`}>
            {uptimePercent}%
          </span>
          <div class={styles.rangeButtons}>
            {RANGES.map((r) => (
              <button
                key={r.key}
                type="button"
                onClick={() => setRange(r.key)}
                aria-pressed={range === r.key}
                class={`${styles.rangeButton} ${range === r.key ? styles.rangeButtonActive : ''}`}
              >
                {r.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* a11y [WCAG 1.1.1]: accessible summary for AT; segments are decorative detail */}
      <div
        class={styles.track}
        role="img"
        aria-label={`Uptime timeline: ${uptimePercent}% uptime over the last ${selectedRange.label}`}
        onMouseLeave={() => setHoveredIdx(null)}
      >
        {segments.map((status, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredIdx(i)}
            class={`${styles.segment} ${SEGMENT_STYLES[status] || styles.segmentNone}`}
            title={`${new Date(now - rangeMs + i * segmentMs).toLocaleString()} - ${status}`}
          />
        ))}
      </div>

      {hoveredIdx !== null && (
        <div class={styles.tooltip}>
          {new Date(now - rangeMs + hoveredIdx * segmentMs).toLocaleString()} - {segments[hoveredIdx]}
        </div>
      )}
    </div>
  );
}
