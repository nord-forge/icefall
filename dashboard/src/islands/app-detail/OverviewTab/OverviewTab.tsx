import { useEffect, useState } from 'preact/hooks';
import type { App, Deploy, HealthCheckResult } from '@lib/types';
import { api } from '@lib/api';
import { formatRelativeTime, shortSha, formatDuration } from '@lib/format';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import UptimeTimeline from '@islands/shared/UptimeTimeline/UptimeTimeline';
import DriftBanner from '@islands/app-detail/DriftBanner/DriftBanner';
import GhostModeSection from './components/GhostModeSection';
import InstancesSummary from './components/InstancesSummary';
import styles from './overview-tab.module.css';

type Props = {
  app: App;
}

export default function OverviewTab({ app }: Props) {
  const [deploys, setDeploys] = useState<Deploy[]>([]);
  const [healthResults, setHealthResults] = useState<HealthCheckResult[]>([]);

  useEffect(() => {
    api.listDeploys(app.id).then(({ data }) => setDeploys(data.slice(0, 5))).catch(() => {});
    api.getHealth(app.id).then(({ data }) => setHealthResults(data)).catch(() => {});
  }, [app.id]);

  const latestDeploy = deploys[0];
  const healthCheck = healthResults[0];
  const healthStatus = healthCheck?.current_status || 'unknown';
  const isImageApp = !!app.image_ref;
  const isComposeApp = !!app.compose_content;
  const isNativeApp = app.deploy_mode === 'native';

  /** Parse service names from compose YAML for display. */
  const composeServices: string[] = (() => {
    if (!app.compose_content) return [];
    try {
      const lines = app.compose_content.split('\n');
      let inServices = false;
      const names: string[] = [];
      for (const line of lines) {
        if (/^services:\s*$/.test(line)) {
          inServices = true;
          continue;
        }
        if (inServices) {
          const match = line.match(/^  ([a-zA-Z0-9_-]+):\s*$/);
          if (match) names.push(match[1]);
          if (/^[a-zA-Z]/.test(line) && !line.startsWith(' ')) inServices = false;
        }
      }
      return names;
    } catch {
      return [];
    }
  })();

  return (
    <div class={styles.grid}>
      <DriftBanner appId={app.id} />
      <div class={styles.panel}>
        <h3 class={styles.sectionTitle}>
          Status
        </h3>
        <div class={styles.statusRow}>
          <StatusDot status={latestDeploy?.status === 'running' ? 'online' : latestDeploy?.status || 'stopped'} />
          <span class={styles.statusText}>
            {latestDeploy?.status === 'running' ? `Running ${latestDeploy.started_at ? formatRelativeTime(latestDeploy.started_at) : ''}` : latestDeploy?.status || 'No deploys yet'}
          </span>
          {isComposeApp && (
            <span class={styles.composeBadge}>Compose Stack</span>
          )}
          {isNativeApp && (
            <span class={styles.nativeBadge}>Native</span>
          )}
          {!isNativeApp && !isComposeApp && app.deploy_mode === 'container' && (
            <span class={styles.containerBadge}>Container</span>
          )}
        </div>

        {latestDeploy?.container_id && !isComposeApp && !isNativeApp && (
          <div class={styles.detailBlock}>
            <span class={styles.detailLabel}>Container ID</span>
            <div class={styles.detailValue}>
              {latestDeploy.container_id.slice(0, 12)}
            </div>
          </div>
        )}

        {!isComposeApp && !isNativeApp && (
          <div class={styles.detailBlock}>
            <span class={styles.detailLabel}>Port</span>
            <div class={styles.detailValueBase}>3000</div>
          </div>
        )}

        {isNativeApp && (
          <div class={styles.detailBlock}>
            <span class={styles.detailLabel}>Serving</span>
            <div class={styles.detailValueBase}>Static files via Caddy</div>
          </div>
        )}

        {isComposeApp && composeServices.length > 0 && (
          <div class={styles.detailBlock}>
            <span class={styles.detailLabel}>Services ({composeServices.length})</span>
            <ul class={styles.composeServiceList} aria-label="Compose services">
              {composeServices.map((name) => (
                <li key={name} class={styles.composeServiceItem}>{name}</li>
              ))}
            </ul>
          </div>
        )}

        {isImageApp && (
          <div class={styles.detailBlock}>
            <span class={styles.detailLabel}>Image</span>
            <div class={styles.detailValue}>
              {app.image_ref}
            </div>
          </div>
        )}

        {!isImageApp && !isComposeApp && latestDeploy?.image_ref && (
          <div class={styles.detailBlock}>
            <span class={styles.detailLabel}>Build Image</span>
            <div class={styles.detailValue}>
              {latestDeploy.image_ref}
            </div>
          </div>
        )}
      </div>

      <InstancesSummary appId={app.id} desiredInstances={app.desired_instances} />

      <div class={styles.panel}>
        <h3 class={styles.sectionTitle}>Health</h3>
        {!healthCheck ? (
          <p class={styles.emptyText}>No health check configured</p>
        ) : (
          <>
            <div class={styles.statusRow}>
              <span class={`${styles.healthBadge} ${styles[`health_${healthStatus}`] || ''}`}>
                {healthStatus === 'healthy' ? 'Healthy' : healthStatus === 'unhealthy' ? 'Unhealthy' : 'Unknown'}
              </span>
              {healthCheck.uptime_percent >= 0 && (
                <span class={styles.uptimeText}>{healthCheck.uptime_percent.toFixed(1)}% uptime</span>
              )}
            </div>
            <div class={styles.detailBlock}>
              <span class={styles.detailLabel}>Type</span>
              <div class={styles.detailValueBase}>{healthCheck.check.check_type.toUpperCase()}</div>
            </div>
            <div class={styles.detailBlock}>
              <span class={styles.detailLabel}>Interval</span>
              <div class={styles.detailValueBase}>{healthCheck.check.interval_secs}s</div>
            </div>
            <div class={styles.detailBlock}>
              <span class={styles.detailLabel}>Auto-restart</span>
              <div class={styles.detailValueBase}>{healthCheck.check.auto_restart ? 'Enabled' : 'Disabled'}</div>
            </div>
            {healthCheck.recent_events.length > 0 && (
              <div style={{ marginTop: 'var(--space-3)' }}>
                <span class={styles.detailLabel}>Recent Events</span>
                <div class={styles.healthEvents}>
                  {healthCheck.recent_events.slice(0, 5).map((event) => (
                    <div key={event.id} class={styles.healthEvent}>
                      <span class={`${styles.healthDot} ${event.status === 'healthy' ? styles.healthDotGreen : styles.healthDotRed}`} />
                      <span class={styles.detailValue}>{event.status}</span>
                      <span class={styles.timeCell}>{formatRelativeTime(event.checked_at)}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </div>

      <div class={styles.deploysPanel}>
        <h3 class={styles.sectionTitleFlush}>
          Recent Deploys
        </h3>
        {deploys.length === 0 ? (
          <p class={styles.emptyText}>No deploys yet</p>
        ) : (
          <table class={styles.deploysTable}>
            <thead>
              <tr class={styles.deploysTheadRow}>
                {isComposeApp ? (
                  <th>Deploy</th>
                ) : isImageApp ? (
                  <th>Image</th>
                ) : (
                  <>
                    <th>Commit</th>
                    <th>Branch</th>
                  </>
                )}
                <th>Status</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {deploys.map((d) => (
                <tr key={d.id} class={styles.deploysRow}>
                  {isComposeApp ? (
                    <td class={styles.deploysCell}>
                      <a href={`/apps/${app.id}/deploys/${d.id}`} class={styles.commitLink}>
                        {d.id.slice(0, 8)}
                      </a>
                    </td>
                  ) : isImageApp ? (
                    <td class={styles.deploysCell}>
                      <a href={`/apps/${app.id}/deploys/${d.id}`} class={styles.commitLink}>
                        {d.image_ref || app.image_ref || '-'}
                      </a>
                    </td>
                  ) : (
                    <>
                      <td class={styles.deploysCell}>
                        <a href={`/apps/${app.id}/deploys/${d.id}`} class={styles.commitLink}>
                          {d.git_sha ? shortSha(d.git_sha) : '-'}
                        </a>
                      </td>
                      <td class={`${styles.deploysCell} ${styles.branchCell}`}>
                        {app.git_branch}
                      </td>
                    </>
                  )}
                  <td class={styles.deploysCell}>
                    <StatusDot status={d.status} />
                  </td>
                  <td class={`${styles.deploysCell} ${styles.timeCell}`}>
                    {formatRelativeTime(d.created_at)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {deploys.length > 0 && (
          <a href={`/apps/${app.id}/deploys`} class={styles.viewAll}>
            View all &rarr;
          </a>
        )}
      </div>

      {app.ghost_mode_enabled && (
        <GhostModeSection app={app} />
      )}

      <div class={styles.uptimePanel}>
        <UptimeTimeline appId={app.id} />
      </div>
    </div>
  );
}
