import { useEffect, useState } from 'preact/hooks';
import { api, request } from '@lib/api';
import Card from '@islands/shared/Card/Card';
import Stat from '@islands/shared/Stat/Stat';
import styles from './analytics-page.module.css';

type Analytics = {
  total_deploys: number;
  successful: number;
  failed: number;
  success_rate: number;
  avg_build_time_secs: number;
  per_app: { app_id: string; deploy_count: number; success_count: number; fail_count: number }[];
};

export default function AnalyticsPage() {
  const [data, setData] = useState<Analytics | null>(null);
  const [days, setDays] = useState(30);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    request<{ data: Analytics }>(`/analytics/deploys?days=${days}`)
      .then(({ data }) => setData(data))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [days]);

  if (loading) return <p class={styles.loading}>Loading analytics...</p>;
  if (!data) return <p class={styles.loading}>No data available.</p>;

  return (
    <div class={styles.page}>
      <div class={styles.header}>
        <h1 class={styles.title}>Deploy analytics</h1>
        <div class={styles.rangeSelector} role="group" aria-label="Time range">
          {[7, 30, 90].map((d) => (
            <button
              key={d}
              type="button"
              class={`${styles.rangeBtn} ${d === days ? styles.active : ''}`}
              onClick={() => setDays(d)}
              aria-pressed={d === days}
            >
              {d}d
            </button>
          ))}
        </div>
      </div>

      <div class={styles.statsGrid}>
        <Card>
          <Stat label="Total deploys" value={data.total_deploys} />
        </Card>
        <Card>
          <Stat label="Success rate" value={`${data.success_rate}%`} />
        </Card>
        <Card>
          <Stat label="Avg build time" value={`${data.avg_build_time_secs}s`} />
        </Card>
        <Card>
          <Stat label="Failed" value={data.failed} />
        </Card>
      </div>

      {data.per_app.length > 0 && (
        <Card title="Per-app breakdown">
          <table class={styles.table}>
            <thead>
              <tr>
                <th class={styles.th}>App</th>
                <th class={styles.th}>Deploys</th>
                <th class={styles.th}>Success</th>
                <th class={styles.th}>Failed</th>
              </tr>
            </thead>
            <tbody>
              {data.per_app.map((app) => (
                <tr key={app.app_id} class={styles.row}>
                  <td class={styles.td}>{app.app_id}</td>
                  <td class={styles.td}>{app.deploy_count}</td>
                  <td class={styles.td}>{app.success_count}</td>
                  <td class={styles.td}>{app.fail_count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}
    </div>
  );
}
