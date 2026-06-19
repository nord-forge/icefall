import { Loader2, PackageCheck, GitFork, FileStack, AlertTriangle } from 'lucide-preact';
import type { RepoDetection } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import styles from '../app-create.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  loading: boolean;
  error: string;
  result: RepoDetection | null;
  baseDirectory: string;
  onPickWorkspace: (dir: string) => void;
  onSwitchToCompose: (file: string) => void;
};

// Human label for the resolved deploy method, used in the summary line.
function deployMethod(result: RepoDetection): string {
  const { framework } = result.detection;
  if (framework === 'dockerfile') return 'Build from Dockerfile';
  if (framework === 'static-site' || framework === 'vite-react' || framework === 'vite-vue')
    return 'Static site';
  return 'Web service';
}

export default function DetectStep({
  loading,
  error,
  result,
  baseDirectory,
  onPickWorkspace,
  onSwitchToCompose,
}: Props) {
  if (loading) {
    return (
      <div class={formStyles.fieldGroup}>
        <p class={styles.detectStatus}>
          <Loader2 size={16} class={styles.spin} aria-hidden="true" />
          Inspecting repository…
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div class={formStyles.fieldGroup}>
        <p class={styles.fieldError} role="alert">
          {error}
        </p>
      </div>
    );
  }

  if (!result) return null;

  const { detection, hints, foreign_coupling } = result;

  // Monorepo with no root app: must pick a workspace before continuing (AC3).
  if (hints.is_monorepo && !baseDirectory) {
    return (
      <div class={formStyles.fieldGroup}>
        <div class={styles.servicePreview}>
          <p class={styles.servicePreviewLabel}>
            <GitFork size={16} aria-hidden="true" /> This looks like a monorepo
          </p>
          <p class={styles.modeHint}>
            No deployable app at the repository root. Pick the workspace to deploy.
          </p>
          <ul class={styles.serviceList}>
            {hints.workspaces.map((ws) => (
              <li key={ws} class={styles.serviceItem}>
                <Button variant="ghost" onClick={() => onPickWorkspace(ws)}>
                  {ws}
                </Button>
              </li>
            ))}
          </ul>
        </div>
      </div>
    );
  }

  // Variant Dockerfiles with no plain Dockerfile: can't auto-pick a target (AC2).
  const ambiguousDockerfiles = hints.dockerfiles.length > 1 && !hints.has_plain_dockerfile;

  return (
    <div class={formStyles.fieldGroup}>
      <div class={styles.reviewGrid}>
        <span class={styles.reviewLabel}>Deploy method</span>
        <span class={styles.reviewValue}>
          <PackageCheck size={14} aria-hidden="true" /> {deployMethod(result)}
        </span>
        <span class={styles.reviewLabel}>Framework</span>
        <span class={styles.reviewValue}>
          {detection.framework}
          {detection.astro_mode ? ` (${detection.astro_mode})` : ''}
        </span>
        <span class={styles.reviewLabel}>Package manager</span>
        <span class={styles.reviewValue}>{detection.package_manager}</span>
        {baseDirectory ? (
          <>
            <span class={styles.reviewLabel}>Directory</span>
            <span class={styles.reviewValue}>{baseDirectory}</span>
          </>
        ) : null}
      </div>

      {hints.compose_files.length > 0 ? (
        <div class={styles.servicePreview}>
          <p class={styles.servicePreviewLabel}>
            <FileStack size={16} aria-hidden="true" /> Compose file found
          </p>
          <p class={styles.modeHint}>
            Deploy as a Compose stack instead of building from source?
          </p>
          {hints.compose_files.map((file) => (
            <Button key={file} variant="ghost" onClick={() => onSwitchToCompose(file)}>
              Use {file}
            </Button>
          ))}
        </div>
      ) : null}

      {ambiguousDockerfiles ? (
        <p class={styles.modeHint}>
          <AlertTriangle size={14} aria-hidden="true" /> Multiple Dockerfiles found
          ({hints.dockerfiles.join(', ')}). Edit the build settings to choose a target,
          or use the compose file above.
        </p>
      ) : null}

      {foreign_coupling ? (
        <p class={styles.modeHint} role="status">
          <AlertTriangle size={14} aria-hidden="true" /> {foreign_coupling.file} looks
          built for another platform (external network, routing labels, or a proxy
          sidecar). You can clean it up after switching to Compose.
        </p>
      ) : null}
    </div>
  );
}
