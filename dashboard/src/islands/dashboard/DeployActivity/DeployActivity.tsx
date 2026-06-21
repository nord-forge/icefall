import { useEffect, useState } from 'preact/hooks';
import { api, request } from '@lib/api';
import { createSSEClient } from '@lib/sse';
import { formatRelativeTime, formatDuration } from '@lib/format';
import type { App, Deploy } from '@lib/types';
import Card from '@islands/shared/Card/Card';
import Stat from '@islands/shared/Stat/Stat';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import EmptyState from '@islands/shared/EmptyState/EmptyState';
import { Rocket } from 'lucide-preact';
import styles from './deploy-activity.module.css';

type Analytics = {
  total_deploys: number;
  successful: number;
  failed: number;
  success_rate: number;
  avg_build_time_secs: number;
};

const FEED_LIMIT = 8;

export default function DeployActivity() {
  const [analytics, setAnalytics] = useState<Analytics | null>(null);
  const [deploys, setDeploys] = useState<Deploy[]>([]);
  const [names, setNames] = useState<Record<string, string>>({});
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let active = true;

    async function load() {
      // Velocity stats — independent of the feed
      request<{ data: Analytics }>('/analytics/deploys?days=7')
        .then(({ data }) => { if (active) setAnalytics(data); })
        .catch(() => {});

      // Recent deploys feed (one latest deploy per app)
      try {
        const { data: apps } = await api.listApps();
        const nameMap: Record<string, string> = {};
        for (const app of apps as App[]) nameMap[app.id] = app.name;
        if (active) setNames(nameMap);

        if (apps.length > 0) {
          const { data: latest } = await api.getLatestDeploys(apps.map((a) => a.id));
          latest.sort((a, b) => (b.created_at || '').localeCompare(a.created_at || ''));
          if (active) setDeploys(latest.slice(0, FEED_LIMIT));
        }
      } catch { /* feed stays empty */ }

      if (active) setLoaded(true);
    }

    load();

    const sse = createSSEClient('/api/v1/events', {
      'deploy.status': (raw: any) => {
        if (!raw?.app_id || !raw?.status) return;
        setDeploys((prev) => {
          const next = prev.filter((d) => d.app_id !== raw.app_id);
          const existing = prev.find((d) => d.app_id === raw.app_id);
          const updated: Deploy = existing
            ? { ...existing, status: raw.status }
            : ({ app_id: raw.app_id, status: raw.status, created_at: new Date().toISOString() } as Deploy);
          return [updated, ...next].slice(0, FEED_LIMIT);
        });
      },
    });

    return () => { active = false; sse.close(); };
  }, []);

  const pending = deploys.filter((d) => d.status === 'scheduled' || d.status === 'pending').length;

  return (
    <Card title="Deploy activity">
      <div class={styles.stats}>
        <Stat label="Deploys (7d)" value={analytics ? analytics.total_deploys : '—'} />
        <Stat label="Success rate" value={analytics ? `${Math.round(analytics.success_rate)}%` : '—'} />
        <Stat
          label="Avg build"
          value={analytics && analytics.avg_build_time_secs > 0 ? formatDuration(analytics.avg_build_time_secs) : '—'}
        />
        <Stat label="Awaiting action" value={pending} />
      </div>

      <div class={styles.feed}>
        {!loaded ? (
          [0, 1, 2, 3].map((i) => <div key={i} class={styles.feedSkeleton} aria-hidden="true" />)
        ) : deploys.length === 0 ? (
          <EmptyState icon={Rocket} title="No deploys yet" description="Deploys will appear here as they run." compact />
        ) : (
          <ul class={styles.list}>
            {deploys.map((d) => (
              <li key={d.app_id} class={styles.item}>
                <span class={styles.appName}>{names[d.app_id] || 'Unknown app'}</span>
                <span class={styles.meta}>
                  <StatusDot status={d.status} />
                  {d.created_at && <span class={styles.time}>{formatRelativeTime(d.created_at)}</span>}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Card>
  );
}
