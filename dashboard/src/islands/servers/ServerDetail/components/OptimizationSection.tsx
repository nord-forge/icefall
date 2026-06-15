import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import type { Recommendation, ServerOptimizations } from '@lib/types';
import { Gauge, TrendingDown, TrendingUp, Moon, Server as ServerIcon } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import { addToast } from '@stores/toast';
import styles from './optimization-section.module.css';

type Props = {
  serverId: string;
};

const KIND_META: Record<Recommendation['kind'], { label: string; icon: typeof Gauge }> = {
  over_provisioned: { label: 'Over-provisioned', icon: TrendingDown },
  under_provisioned: { label: 'Under-provisioned', icon: TrendingUp },
  idle: { label: 'Idle', icon: Moon },
  colocation: { label: 'Co-location', icon: ServerIcon },
};

function fmtBytes(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`;
}

export default function OptimizationSection({ serverId }: Props) {
  const [data, setData] = useState<ServerOptimizations | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState<string | null>(null);

  async function load() {
    try {
      const { data } = await api.getServerOptimizations(serverId);
      setData(data);
    } catch (err: any) {
      setError(err?.message || 'Failed to load optimizations');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { load(); }, [serverId]);

  async function applyOne(rec: Recommendation) {
    setBusy(rec.app_id + rec.kind);
    try {
      await api.applyOptimization(serverId, {
        app_id: rec.app_id,
        kind: rec.kind,
        memory_bytes: rec.suggested_memory_bytes ?? undefined,
      });
      addToast('success', `Applied to ${rec.app_name}`);
      await load();
    } catch (err: any) {
      addToast('error', err?.message || 'Failed to apply');
    } finally {
      setBusy(null);
    }
  }

  async function applyAll() {
    setBusy('all');
    try {
      const { message } = await api.applyAllOptimizations(serverId);
      addToast('success', message);
      await load();
    } catch (err: any) {
      addToast('error', err?.message || 'Failed to apply all');
    } finally {
      setBusy(null);
    }
  }

  const Header = (
    <h2 class={styles.title}>
      <Gauge size={18} aria-hidden="true" />
      Optimization
    </h2>
  );

  if (loading) {
    return (
      <div class={styles.container}>
        {Header}
        <p class={styles.info} role="status" aria-live="polite">Analyzing usage…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div class={styles.container}>
        {Header}
        <p class={styles.info} role="status" aria-live="polite">{error}</p>
      </div>
    );
  }

  const recs = data?.recommendations ?? [];
  const autoCount = recs.filter((r) => r.auto_applicable).length;

  if (recs.length === 0) {
    return (
      <div class={styles.container}>
        {Header}
        <p class={styles.info}>
          No optimization opportunities found. Containers are right-sized based on the
          last {data?.analysis_days ?? 7} days of usage.
        </p>
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <div class={styles.headerRow}>
        {Header}
        {autoCount > 0 && (
          <Button variant="secondary" size="sm" onClick={applyAll} loading={busy === 'all'}>
            Apply all ({autoCount})
          </Button>
        )}
      </div>

      {data && data.summary.ram_saved_bytes > 0 && (
        <p class={styles.summary} role="status" aria-live="polite">
          Right-sizing could free <strong>{fmtBytes(data.summary.ram_saved_bytes)}</strong> of RAM
          (~${data.summary.estimated_monthly_savings_usd.toFixed(2)}/mo).
        </p>
      )}

      <ul class={styles.list}>
        {recs.map((rec) => {
          const meta = KIND_META[rec.kind];
          const Icon = meta.icon;
          return (
            <li key={rec.app_id + rec.kind} class={styles.card}>
              <div class={styles.cardMain}>
                <span class={styles.kindBadge}>
                  <Icon size={13} aria-hidden="true" /> {meta.label}
                </span>
                <span class={styles.appName}>{rec.app_name}</span>
                <p class={styles.message}>{rec.message}</p>
                {rec.suggested_memory_bytes !== null && (
                  <p class={styles.change}>
                    <span class={styles.current}>{fmtBytes(rec.current_memory_bytes)}</span>
                    {' → '}
                    <span class={styles.suggested}>{fmtBytes(rec.suggested_memory_bytes)}</span>
                    {rec.ram_saved_bytes > 0 && (
                      <span class={styles.saved}> (saves {fmtBytes(rec.ram_saved_bytes)})</span>
                    )}
                  </p>
                )}
              </div>
              {rec.auto_applicable ? (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => applyOne(rec)}
                  loading={busy === rec.app_id + rec.kind}
                >
                  Apply
                </Button>
              ) : (
                <span class={styles.advisory}>Advisory</span>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
