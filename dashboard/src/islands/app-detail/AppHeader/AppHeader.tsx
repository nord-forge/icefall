import type { App, DeployStatus, Server } from '@lib/types';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import Button from '@islands/shared/Button/Button';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { useState } from 'preact/hooks';
import { Settings, Rocket, GitBranch, Container, Layers, Square, Play, RotateCw } from 'lucide-preact';
import styles from './app-header.module.css';

type Props = {
  app: App;
  status?: DeployStatus | 'online';
  onStatusChange?: () => void;
  serverName?: string;
  serverCount?: number;
}

export default function AppHeader({ app, status, onStatusChange, serverName, serverCount = 1 }: Props) {
  const [deploying, setDeploying] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [starting, setStarting] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [optimisticStatus, setOptimisticStatus] = useState<DeployStatus | 'online' | null>(null);

  const displayStatus = optimisticStatus ?? status;
  const isRunning = displayStatus === 'running' || displayStatus === 'online';
  const isStopped = displayStatus === 'stopped';

  async function handleDeploy() {
    setDeploying(true);
    // Optimistic: show deploying status immediately
    setOptimisticStatus('deploying');
    try {
      await api.triggerDeploy(app.id);
      window.location.href = `/apps/${app.id}/deploys`;
    } catch (err: any) {
      // Revert optimistic status
      setOptimisticStatus(null);
      addToast('error', err.message || 'Failed to trigger deploy');
      setDeploying(false);
    }
  }

  async function handleStop() {
    setStopping(true);
    const prevStatus = status;
    // Optimistic: show stopped immediately
    setOptimisticStatus('stopped');
    try {
      await api.stopApp(app.id);
      onStatusChange?.();
      setOptimisticStatus(null);
    } catch (err: any) {
      // Revert to previous status
      setOptimisticStatus(null);
      addToast('error', err.message || 'Failed to stop app');
    }
    setStopping(false);
  }

  async function handleStart() {
    setStarting(true);
    // Optimistic: show running immediately
    setOptimisticStatus('running');
    try {
      await api.startApp(app.id);
      onStatusChange?.();
      setOptimisticStatus(null);
    } catch (err: any) {
      // Revert to previous status
      setOptimisticStatus(null);
      addToast('error', err.message || 'Failed to start app');
    }
    setStarting(false);
  }

  async function handleRestart() {
    setRestarting(true);
    // Optimistic: show deploying briefly (restart in progress)
    setOptimisticStatus('deploying');
    try {
      await api.restartApp(app.id);
      onStatusChange?.();
      setOptimisticStatus(null);
    } catch (err: any) {
      // Revert to previous status
      setOptimisticStatus(null);
      addToast('error', err.message || 'Failed to restart app');
    }
    setRestarting(false);
  }

  return (
    <div class={styles.header}>
      <div>
        {/* a11y [WCAG 2.4.8]: proper breadcrumb navigation */}
        <nav aria-label="Breadcrumb" class={styles.breadcrumb}>
          <a href="/">Apps</a> / <span aria-current="page">{app.name}</span>
        </nav>
        <div class={styles.titleRow}>
          <h1 class={styles.title}>{app.name}</h1>
          <StatusDot status={displayStatus || 'online'} />
        </div>
        {serverCount > 1 && serverName && (
          <span class={styles.serverIndicator}>
            on <a href={`/servers/${app.server_id}`} class={styles.serverLink}>{serverName}</a>
          </span>
        )}
        <div class={styles.meta}>
          {app.compose_content ? (
            <span class={styles.branchInfo}>
              <Layers size={14} aria-hidden="true" />
              <span class={styles.mono}>Compose Stack</span>
            </span>
          ) : app.image_ref ? (
            <span class={styles.branchInfo}>
              <Container size={14} aria-hidden="true" />
              <span class={styles.mono}>{app.image_ref}</span>
            </span>
          ) : (
            <>
              {app.git_repo && (
                <span class={styles.branchInfo}>
                  <GitBranch size={14} aria-hidden="true" />
                  <span class={styles.mono}>{app.git_branch}</span>
                </span>
              )}
              {app.git_repo && (
                <span class={styles.repoUrl}>
                  {app.git_repo.replace(/^https?:\/\//, '')}
                </span>
              )}
            </>
          )}
        </div>
      </div>
      <div class={styles.actions}>
        {isStopped && (
          <Button variant="secondary" onClick={handleStart} loading={starting}>
            <Play size={14} /> Start
          </Button>
        )}
        {isRunning && (
          <Button variant="secondary" onClick={handleRestart} loading={restarting}>
            <RotateCw size={14} /> Restart
          </Button>
        )}
        {isRunning && (
          <Button variant="secondary" onClick={handleStop} loading={stopping}>
            <Square size={14} /> Stop
          </Button>
        )}
        <a href={`/apps/${app.id}/settings`} class={styles.settingsLink}>
          {/* a11y [4.1.2]: button label provided by visible text content */}
          <Button variant="secondary">
            <Settings size={14} /> Settings
          </Button>
        </a>
        <Button variant="primary" onClick={handleDeploy} loading={deploying}>
          <Rocket size={14} /> Deploy
        </Button>
      </div>
    </div>
  );
}
