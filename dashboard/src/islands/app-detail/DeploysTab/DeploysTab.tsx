import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { Deploy } from '@lib/types';
import { formatRelativeTime, formatDuration, shortSha } from '@lib/format';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import Button from '@islands/shared/Button/Button';
import { RotateCcw } from 'lucide-preact';
import styles from './deploys-tab.module.css';

type Props = {
  appId: string;
}

export default function DeploysTab({ appId }: Props) {
  const [deploys, setDeploys] = useState<Deploy[]>([]);
  const [loading, setLoading] = useState(true);
  const [rollingBack, setRollingBack] = useState('');

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
    api.listDeploys(appId).then(({ data }) => { setDeploys(data); setLoading(false); }).catch(() => setLoading(false));
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
                <td class={styles.cell}><StatusDot status={d.status} /></td>
                <td class={`${styles.cell} ${styles.duration}`}>
                  {duration ? formatDuration(duration) : '-'}
                </td>
                <td class={`${styles.cell} ${styles.time}`}>
                  {formatRelativeTime(d.created_at)}
                </td>
                <td class={styles.cell}>
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
    </div>
  );
}
