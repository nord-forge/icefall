import type { App } from '@lib/types';
import { formatRelativeTime } from '@lib/format';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import Button from '@islands/shared/Button/Button';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { Rocket, RotateCcw, Play } from 'lucide-preact';
import { useState } from 'preact/hooks';
import styles from './app-card.module.css';

type Props = {
  app: App;
  latestDeployStatus?: string;
  latestDeployTime?: string;
  serverName?: string;
}

export default function AppCard({ app, latestDeployStatus, latestDeployTime, serverName }: Props) {
  const [deploying, setDeploying] = useState(false);
  const [optimisticStatus, setOptimisticStatus] = useState<string | null>(null);
  const status = optimisticStatus || latestDeployStatus || 'stopped';
  const isOnline = status === 'running';
  const isFailed = status === 'failed';
  const isBuilding = status === 'building' || status === 'deploying';
  const isStopped = status === 'stopped';

  async function handleDeploy() {
    setDeploying(true);
    // Optimistic: show deploying status immediately
    setOptimisticStatus('deploying');
    try {
      await api.triggerDeploy(app.id);
      window.location.href = `/apps/${app.id}/deploys`;
    } catch (err: any) {
      setOptimisticStatus(null);
      addToast('error', err.message || 'Failed to trigger deploy');
      setDeploying(false);
    }
  }

  {/* a11y [WCAG 4.1.2]: no nested interactive elements — link covers header, buttons are siblings */}
  return (
    <div class={styles.card}>
      <a href={`/apps/${app.id}`} class={styles.cardLink}>
        <div class={styles.header}>
          <span class={styles.name}>
            {app.name}
          </span>
          <StatusDot status={isOnline ? 'online' : status} />
        </div>

        <div class={styles.badges}>
          {app.deploy_mode === 'native' && (
            <span class={styles.modeBadge}>Static</span>
          )}
          {app.image_ref && !app.compose_content && (
            <span class={styles.modeBadge}>Image</span>
          )}
          {app.compose_content && (
            <span class={styles.modeBadge}>Compose</span>
          )}
        </div>

        {app.git_repo && (
          <span class={styles.repo}>
            {app.git_repo.replace(/^https?:\/\//, '').replace(/\.git$/, '')}
          </span>
        )}

        {latestDeployTime && (
          <span class={styles.time}>
            {formatRelativeTime(latestDeployTime)}
          </span>
        )}

        {app.tags && (
          <div class={styles.tags}>
            {app.tags.split(',').map((t) => t.trim()).filter(Boolean).map((tag) => (
              <span key={tag} class={styles.tag}>{tag}</span>
            ))}
          </div>
        )}

        {serverName && (
          <span class={styles.serverLabel}>{serverName}</span>
        )}
      </a>

      <div class={styles.actions}>
        {isOnline && (
          <Button variant="primary" size="md" fullWidth onClick={handleDeploy} loading={deploying}>
            <Rocket size={14} /> Deploy
          </Button>
        )}
        {isFailed && (
          <Button variant="secondary" size="md" fullWidth onClick={handleDeploy} loading={deploying}>
            <RotateCcw size={14} /> Redeploy
          </Button>
        )}
        {isStopped && (
          <Button variant="secondary" size="md" fullWidth onClick={handleDeploy} loading={deploying}>
            <Play size={14} /> Start
          </Button>
        )}
        {isBuilding && (
          <Button variant="secondary" size="md" fullWidth disabled>
            <span class={styles.spinner} aria-hidden="true" />
            Deploying...
          </Button>
        )}
      </div>
    </div>
  );
}
