import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import { mapLimit } from '@lib/concurrency';
import { formatBytes, formatRelativeTime } from '@lib/format';
import Card from '@islands/shared/Card/Card';
import { Sparkles, Trash2, DatabaseBackup } from 'lucide-preact';
import type { LucideIcon } from 'lucide-preact';
import styles from './housekeeping.module.css';

const BACKUP_STALE_DAYS = 7;
const CONCURRENCY = 4;
const DAY_MS = 86_400_000;

type Row = {
  key: string;
  icon: LucideIcon;
  label: string;
  value: string;
  href: string;
};

export default function Housekeeping() {
  const [rows, setRows] = useState<Row[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let active = true;

    async function load() {
      const next: Row[] = [];

      // Optimization recommendations across all servers
      try {
        const { data: servers } = await api.listServers();
        const opts = await mapLimit(servers, CONCURRENCY, (s) => api.getServerOptimizations(s.id));
        let count = 0;
        let savings = 0;
        for (const o of opts) {
          if (!o) continue;
          count += o.data.summary.count;
          savings += o.data.summary.estimated_monthly_savings_usd;
        }
        next.push({
          key: 'optimizations',
          icon: Sparkles,
          label: 'Optimization tips',
          value: count === 0 ? 'None' : `${count} · save ~$${Math.round(savings)}/mo`,
          href: '/servers',
        });
      } catch { /* skip row */ }

      // Last cleanup run
      try {
        const { data: history } = await api.listCleanupHistory();
        const last = history.find((r) => r.status === 'completed') || history[0];
        next.push({
          key: 'cleanup',
          icon: Trash2,
          label: 'Last cleanup',
          value: last
            ? `${formatBytes(last.freed_bytes)} freed · ${formatRelativeTime(last.started_at)}`
            : 'Never run',
          href: '/settings',
        });
      } catch { /* skip row */ }

      // Databases without a recent backup
      try {
        const { data: dbs } = await api.listDatabases();
        const cutoff = Date.now() - BACKUP_STALE_DAYS * DAY_MS;
        const checks = await mapLimit(dbs, CONCURRENCY, async (db: any) => {
          const { data: backups } = await api.listDatabaseBackups(db.id);
          const fresh = backups.some((b) => new Date(b.created_at).getTime() >= cutoff);
          return fresh;
        });
        const stale = checks.filter((c) => c === false).length;
        next.push({
          key: 'backups',
          icon: DatabaseBackup,
          label: 'Backups',
          value: dbs.length === 0
            ? 'No databases'
            : stale === 0 ? 'All recent' : `${stale} stale (>${BACKUP_STALE_DAYS}d)`,
          href: '/databases',
        });
      } catch { /* skip row */ }

      if (active) { setRows(next); setLoaded(true); }
    }

    load();
    return () => { active = false; };
  }, []);

  if (loaded && rows.length === 0) return null;

  return (
    <Card title="Housekeeping">
      <dl class={styles.list}>
        {!loaded
          ? [0, 1, 2].map((i) => <div key={i} class={styles.skeleton} aria-hidden="true" />)
          : rows.map(({ key, icon: Icon, label, value, href }) => (
              <a key={key} href={href} class={styles.row}>
                <span class={styles.rowLabel}>
                  <Icon size={16} aria-hidden="true" class={styles.icon} />
                  <dt>{label}</dt>
                </span>
                <dd class={styles.value}>{value}</dd>
              </a>
            ))}
      </dl>
    </Card>
  );
}
