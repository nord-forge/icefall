import { useEffect, useState } from 'preact/hooks';
import { api, request } from '@lib/api';
import { mapLimit } from '@lib/concurrency';
import { AlertTriangle, HeartPulse, MoonStar, HardDrive, CheckCircle2 } from 'lucide-preact';
import type { LucideIcon } from 'lucide-preact';
import styles from './attention-row.module.css';

const FORECAST_WARN_DAYS = 14;
const HEALTH_CONCURRENCY = 6;
const FORECAST_CONCURRENCY = 4;

type Severity = 'critical' | 'warning';

type AttentionCard = {
  key: string;
  icon: LucideIcon;
  severity: Severity;
  title: string;
  detail: string;
  href: string;
};

type Incident = { id: string; status: string; severity: string };

export default function AttentionRow() {
  const [cards, setCards] = useState<AttentionCard[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let active = true;

    async function load() {
      const found: AttentionCard[] = [];

      // Open incidents
      try {
        const { data } = await request<{ data: Incident[] }>('/incidents');
        const open = data.filter((i) => i.status !== 'resolved');
        if (open.length > 0) {
          const critical = open.filter((i) => i.severity === 'critical').length;
          found.push({
            key: 'incidents',
            icon: AlertTriangle,
            severity: critical > 0 ? 'critical' : 'warning',
            title: `${open.length} open incident${open.length === 1 ? '' : 's'}`,
            detail: critical > 0 ? `${critical} critical` : 'None critical',
            href: '/incidents',
          });
        }
      } catch { /* skip card */ }

      // Inactive apps
      try {
        const { count } = await api.listInactiveApps();
        if (count > 0) {
          found.push({
            key: 'inactive',
            icon: MoonStar,
            severity: 'warning',
            title: `${count} inactive app${count === 1 ? '' : 's'}`,
            detail: 'No recent deploys or traffic',
            href: '/',
          });
        }
      } catch { /* skip card */ }

      // Failing health checks (per-app fan-out, bounded concurrency)
      try {
        const { data: apps } = await api.listApps();
        const healthResults = await mapLimit(apps, HEALTH_CONCURRENCY, (app) => api.getHealth(app.id));
        let unhealthy = 0;
        for (const res of healthResults) {
          if (!res) continue;
          for (const check of res.data) {
            if (check.current_status === 'unhealthy') unhealthy++;
          }
        }
        if (unhealthy > 0) {
          found.push({
            key: 'health',
            icon: HeartPulse,
            severity: 'critical',
            title: `${unhealthy} failing health check${unhealthy === 1 ? '' : 's'}`,
            detail: 'Apps reporting unhealthy',
            href: '/',
          });
        }
      } catch { /* skip card */ }

      // Capacity forecast alerts (per-server fan-out)
      try {
        const { data: servers } = await api.listServers();
        const forecasts = await mapLimit(servers, FORECAST_CONCURRENCY, async (s) => ({
          server: s,
          forecast: (await api.getServerForecast(s.id)).data,
        }));
        let soonest: number | null = null;
        let resource = '';
        let serverName = '';
        for (const f of forecasts) {
          if (!f) continue;
          for (const [label, metric] of [['disk', f.forecast.disk], ['memory', f.forecast.memory]] as const) {
            const days = metric.days_until_full;
            if (days != null && days <= FORECAST_WARN_DAYS && (soonest == null || days < soonest)) {
              soonest = days;
              resource = label;
              serverName = f.server.name;
            }
          }
        }
        if (soonest != null) {
          found.push({
            key: 'forecast',
            icon: HardDrive,
            severity: soonest <= 3 ? 'critical' : 'warning',
            title: `${resource === 'disk' ? 'Disk' : 'Memory'} full in ~${soonest}d`,
            detail: serverName,
            href: '/servers',
          });
        }
      } catch { /* skip card */ }

      if (active) {
        setCards(found);
        setLoaded(true);
      }
    }

    load();
    return () => { active = false; };
  }, []);

  if (!loaded) {
    return (
      <div class={styles.row} aria-hidden="true">
        {[0, 1, 2].map((i) => <div key={i} class={styles.skeleton} />)}
      </div>
    );
  }

  if (cards.length === 0) {
    return (
      <div class={`${styles.card} ${styles.allClear}`}>
        <CheckCircle2 size={20} aria-hidden="true" class={styles.allClearIcon} />
        <div>
          <p class={styles.title}>All systems nominal</p>
          <p class={styles.detail}>No incidents, health failures, or capacity risks.</p>
        </div>
      </div>
    );
  }

  return (
    <div class={styles.row}>
      {cards.map(({ key, icon: Icon, severity, title, detail, href }) => (
        <a key={key} href={href} class={`${styles.card} ${styles[severity]}`}>
          <Icon size={20} aria-hidden="true" class={styles.cardIcon} />
          <div class={styles.cardText}>
            <p class={styles.title}>{title}</p>
            <p class={styles.detail}>{detail}</p>
          </div>
        </a>
      ))}
    </div>
  );
}
