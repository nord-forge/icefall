import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { ExternalLink, AlertTriangle, Check, GitFork } from 'lucide-preact';
import Input from '@islands/shared/Input/Input';
import Button from '@islands/shared/Button/Button';
import RepoBrowser from './RepoBrowser';
import formStyles from '@styles/form.module.css';
import styles from '../app-create.module.css';

type Props = {
  name: string;
  gitRepo: string;
  gitBranch: string;
  validationErrors: Record<string, string>;
  onUpdate: (field: string, value: string) => void;
};

export default function GitRepoStep({
  name,
  gitRepo,
  gitBranch,
  validationErrors,
  onUpdate,
}: Props) {
  const [hasGitHubApp, setHasGitHubApp] = useState<boolean | null>(null);
  const [hasInstallations, setHasInstallations] = useState<boolean | null>(null);
  const [loadingSetup, setLoadingSetup] = useState(false);
  const [browse, setBrowse] = useState(false);
  const [branches, setBranches] = useState<string[]>([]);
  const [loadingBranches, setLoadingBranches] = useState(false);

  // After a repo is picked from the browser, load its branches for the selector.
  async function selectFromBrowser(repo: string, branch: string, installationId: string) {
    onUpdate('git_repo', `https://github.com/${repo}`);
    onUpdate('git_branch', branch);
    onUpdate('github_installation_id', installationId);
    setBrowse(false);
    setLoadingBranches(true);
    try {
      const { data } = await api.listGitSourceBranches(installationId, repo);
      setBranches(data);
    } catch {
      setBranches([]);
    } finally {
      setLoadingBranches(false);
    }
  }

  useEffect(() => {
    api.listGitHubApps()
      .then(({ data }) => setHasGitHubApp(data.length > 0))
      .catch(() => setHasGitHubApp(false));

    api.listGitSources()
      .then(({ data }) => setHasInstallations(data.length > 0))
      .catch(() => setHasInstallations(false));
  }, []);

  const isGitHubUrl = /github\.com/i.test(gitRepo) && gitRepo.trim().length > 0;
  const needsSetup = hasGitHubApp === false;
  const needsInstallation = hasGitHubApp === true && hasInstallations === false;
  const isReady = hasGitHubApp === true && hasInstallations === true;

  async function handleConnectGitHub() {
    setLoadingSetup(true);
    try {
      const setup = await api.getGitHubSetup();
      const form = document.createElement('form');
      form.method = 'POST';
      form.action = setup.form_action;
      form.target = '_self';
      const input = document.createElement('input');
      input.type = 'hidden';
      input.name = 'manifest';
      input.value = JSON.stringify(setup.manifest);
      form.appendChild(input);
      document.body.appendChild(form);
      form.submit();
    } catch (err: any) {
      addToast('error', err.message || 'Failed to start GitHub setup');
      setLoadingSetup(false);
    }
  }

  return (
    <div class={formStyles.fieldGroup}>
      <Input
        label="App Name"
        name="app-name"
        id="create-app-name"
        value={name}
        onChange={(v) => onUpdate('name', v)}
        placeholder="my-awesome-app"
        error={validationErrors.name}
      />
      <Input
        label="Repository URL"
        name="repo-url"
        id="create-repo-url"
        mono
        value={gitRepo}
        onChange={(v) => onUpdate('git_repo', v)}
        placeholder="https://github.com/user/repo"
      />
      {branches.length > 0 ? (
        <div class={formStyles.field}>
          <label htmlFor="create-branch-select" class={formStyles.label}>Branch</label>
          <select
            id="create-branch-select"
            class={formStyles.select}
            value={gitBranch}
            onChange={(e) => onUpdate('git_branch', (e.target as HTMLSelectElement).value)}
          >
            {branches.map((b) => <option key={b} value={b}>{b}</option>)}
          </select>
        </div>
      ) : (
        <Input
          label="Branch"
          name="branch"
          id="create-branch"
          mono
          value={gitBranch}
          helpText={loadingBranches ? 'Loading branches…' : undefined}
          onChange={(v) => onUpdate('git_branch', v)}
        />
      )}

      {isReady && (
        <div>
          <Button variant="secondary" size="sm" onClick={() => setBrowse((b) => !b)}>
            <GitFork size={14} aria-hidden="true" /> {browse ? 'Hide repositories' : 'Browse repositories'}
          </Button>
          {browse && <RepoBrowser onSelect={selectFromBrowser} />}
        </div>
      )}

      {isGitHubUrl && hasGitHubApp !== null && needsSetup && (
        <div class={styles.githubNotice} role="status">
          <GitFork size={18} aria-hidden="true" class={styles.githubNoticeIcon} />
          <div>
            <p class={styles.githubNoticeTitle}>
              Deploying a private repo?
            </p>
            <p class={styles.githubNoticeText}>
              Public repos work out of the box. For private repos, connect a GitHub App to grant Icefall read access. Takes about 30 seconds.
            </p>
            <Button variant="secondary" size="sm" onClick={handleConnectGitHub} loading={loadingSetup}>
              <GitFork size={14} /> Connect GitHub
            </Button>
          </div>
        </div>
      )}

      {isGitHubUrl && hasGitHubApp !== null && needsInstallation && (
        <div class={styles.githubNotice} role="status">
          <AlertTriangle size={18} aria-hidden="true" class={styles.githubNoticeIcon} />
          <div>
            <p class={styles.githubNoticeTitle}>
              GitHub App connected — install it to enable private repos
            </p>
            <p class={styles.githubNoticeText}>
              Your GitHub App is ready but hasn't been installed on an account yet.
              Install it to grant Icefall access to your private repositories.
            </p>
            <a href="/settings">
              <Button variant="secondary" size="sm">
                <ExternalLink size={14} /> Go to settings
              </Button>
            </a>
          </div>
        </div>
      )}

      {isGitHubUrl && isReady && (
        <div class={styles.githubReady} role="status">
          <Check size={16} aria-hidden="true" />
          <span>GitHub integration active — private repos are supported.</span>
        </div>
      )}
    </div>
  );
}
