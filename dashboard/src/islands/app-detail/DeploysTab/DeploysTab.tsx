import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { App, Deploy } from '@lib/types';
import { formatRelativeTime, formatDuration, formatCountdown, shortSha } from '@lib/format';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import Button from '@islands/shared/Button/Button';
import { createSSEClient } from '@lib/sse';
import { RotateCcw, X, CalendarClock } from 'lucide-preact';
import ApprovalBadge from './components/ApprovalBadge';
import CanaryResultsSection from './components/CanaryResultsSection';
import ScheduleDeployDialog from '@islands/app-detail/AppHeader/components/ScheduleDeployDialog';
import { currentTimeZone, loadTimeZone, formatInTimeZone } from '@lib/timezone';
import styles from './deploys-tab.module.css';

type Props = {
  appId: string;
  requireDeployApproval?: boolean;
  canaryEnabled?: boolean;
}

export default function DeploysTab({ appId, requireDeployApproval = false, canaryEnabled = false }: Props) {
  const [deploys, setDeploys] = useState<Deploy[]>([]);
  const [loading, setLoading] = useState(true);
  const [rollingBack, setRollingBack] = useState('');
  const [cancelling, setCancelling] = useState('');
  const [rescheduleTarget, setRescheduleTarget] = useState<Deploy | null>(null);
  const [rescheduling, setRescheduling] = useState(false);
  const [tz, setTz] = useState(currentTimeZone());
  // Drives the scheduled-deploy countdown re-render.
  const [, setTick] = useState(0);

  async function handleCancel(deployId: string) {
    setCancelling(deployId);
    try {
      await api.cancelDeploy(deployId);
      const { data } = await api.listDeploys(appId);
      setDeploys(data);
      addToast('info', 'Deploy cancelled');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to cancel deploy');
    }
    setCancelling('');
  }

  async function handleReschedule(isoUtc: string) {
    if (!rescheduleTarget) return;
    setRescheduling(true);
    try {
      await api.rescheduleDeploy(rescheduleTarget.id, isoUtc);
      const { data } = await api.listDeploys(appId);
      setDeploys(data);
      setRescheduleTarget(null);
      addToast('success', 'Deploy rescheduled');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to reschedule deploy');
    }
    setRescheduling(false);
  }

  async function handleRollback(deployId: string) {
    setRollingBack(deployId);
    // Optimistic: update the status of the rolled-back deploy to 'deploying'
    const prevDeploys = deploys;
    setDeploys(prev => prev.map(d =>
      d.id === deployId ? { ...d, status: 'deploying' as const } : d
    ));
    try {
      await api.rollbackDeploy(appId, deployId);
      const { data } = await api.listDeploys(appId);
      setDeploys(data);
    } catch (err: any) {
      // Revert optimistic update
      setDeploys(prevDeploys);
      addToast('error', err.message || 'Rollback failed');
    }
    setRollingBack('');
  }

  useEffect(() => {
    loadTimeZone().then(setTz).catch(() => {});
    api.listDeploys(appId).then(({ data }) => { setDeploys(data); setLoading(false); }).catch(() => setLoading(false));

    const sse = createSSEClient('/api/v1/events', {
      'deploy.status': () => {
        api.listDeploys(appId).then(({ data }) => setDeploys(data)).catch(() => {});
      },
      'deploy.created': () => {
        api.listDeploys(appId).then(({ data }) => setDeploys(data)).catch(() => {});
      },
    });

    // Refresh scheduled-deploy countdowns once a minute.
    const ticker = window.setInterval(() => setTick((t) => t + 1), 60_000);

    return () => { sse.close(); window.clearInterval(ticker); };
  }, [appId]);

  if (loading) return <p class={styles.message}>Loading deploys...</p>;

  if (deploys.length === 0) return <p class={styles.message}>No deploys yet.</p>;

  const latestRunning = deploys.find((d) => d.status === 'running');

  return (
    <div class={styles.wrapper}>
      <table class={styles.table}>
        <thead>
          <tr class={styles.theadRow}>
            {['Deploy', 'Commit', 'Branch', 'Status', 'Duration', 'Time'].map((h) => (
              <th key={h} class={styles.th}>
                {h}
              </th>
            ))}
            {/* a11y [WCAG 1.3.1]: visually hidden label for actions column */}
            <th class={styles.th}>
              <span class="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {deploys.map((d, i) => {
            const duration = d.started_at && d.finished_at
              ? (new Date(d.finished_at).getTime() - new Date(d.started_at).getTime()) / 1000
              : null;
            const isLast = i === deploys.length - 1;
            const canRollback = d.image_ref && d.status !== 'running' && d.status !== 'pending' && d.status !== 'building' && d.status !== 'deploying' && latestRunning?.id !== d.id;
            return (
              <tr key={d.id} class={isLast ? styles.rowLast : styles.row}>
                <td class={styles.cell}>
                  <a href={`/apps/${appId}/deploys/${d.id}`} class={styles.deployLink}>
                    #{d.id.slice(0, 8)}
                  </a>
                </td>
                <td class={`${styles.cell} ${styles.commitSha}`}>
                  {d.git_sha ? shortSha(d.git_sha) : '-'}
                </td>
                <td class={`${styles.cell} ${styles.mono}`}>main</td>
                <td class={styles.cell}>
                  <StatusDot status={d.status} />
                  {requireDeployApproval && d.status === 'pending' && (
                    <ApprovalBadge deployId={d.id} status={d.status} requiresApproval={requireDeployApproval} />
                  )}
                </td>
                <td class={`${styles.cell} ${styles.duration}`}>
                  {duration ? formatDuration(duration) : '-'}
                </td>
                <td class={`${styles.cell} ${styles.time}`}>
                  {d.status === 'scheduled' && d.scheduled_at ? (
                    <span class={styles.scheduledTime} title={formatInTimeZone(d.scheduled_at, tz)}>
                      <CalendarClock size={12} aria-hidden="true" /> {formatCountdown(d.scheduled_at)}
                    </span>
                  ) : (
                    formatRelativeTime(d.created_at)
                  )}
                </td>
                <td class={styles.cell}>
                  {(d.status === 'scheduled' || d.status === 'pending' || d.status === 'building' || d.status === 'deploying') && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleCancel(d.id)}
                      loading={cancelling === d.id}
                    >
                      <X size={12} /> Cancel
                    </Button>
                  )}
                  {d.status === 'scheduled' && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setRescheduleTarget(d)}
                    >
                      <CalendarClock size={12} /> Reschedule
                    </Button>
                  )}
                  {canRollback && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleRollback(d.id)}
                      loading={rollingBack === d.id}
                    >
                      <RotateCcw size={12} /> Rollback
                    </Button>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {canaryEnabled && latestRunning && (
        <CanaryResultsSection deployId={latestRunning.id} canaryEnabled={canaryEnabled} />
      )}
      <ScheduleDeployDialog
        open={rescheduleTarget !== null}
        mode="reschedule"
        loading={rescheduling}
        initialIso={rescheduleTarget?.scheduled_at ?? undefined}
        onConfirm={handleReschedule}
        onCancel={() => setRescheduleTarget(null)}
      />
    </div>
  );
}
