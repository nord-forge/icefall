import { useState } from 'preact/hooks';
import type { Project } from '@lib/types';
import { api } from '@lib/api';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import { FolderOpen, GitBranch } from 'lucide-preact';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  appId: string;
  name: string;
  gitRepo: string;
  gitBranch: string;
  buildCommand: string;
  projects: Project[];
  selectedProjectId: string;
  onNameChange: (v: string) => void;
  onGitRepoChange: (v: string) => void;
  onGitBranchChange: (v: string) => void;
  onBuildCommandChange: (v: string) => void;
  onProjectChange: (v: string) => void;
};

export default function GeneralSettingsCard({
  appId,
  name,
  gitRepo,
  gitBranch,
  buildCommand,
  projects,
  selectedProjectId,
  onNameChange,
  onGitRepoChange,
  onGitBranchChange,
  onBuildCommandChange,
  onProjectChange,
}: Props) {
  // IF-166: load remote branches on demand to populate the branch datalist.
  const [branches, setBranches] = useState<string[]>([]);
  const [loadingBranches, setLoadingBranches] = useState(false);
  const [branchError, setBranchError] = useState('');

  async function loadBranches() {
    setLoadingBranches(true);
    setBranchError('');
    try {
      const { data } = await api.listBranches(appId);
      setBranches(data);
    } catch (e) {
      setBranchError(e instanceof Error ? e.message : 'Could not load branches');
    }
    setLoadingBranches(false);
  }
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>General Settings</h2>
      <div class={formStyles.fieldRow}>
        <Input
          label="App Name"
          name="app-name"
          id="settings-app-name"
          value={name}
          onChange={onNameChange}
        />
        <Input
          label="Git Repository"
          name="git-repo"
          id="settings-git-repo"
          mono
          value={gitRepo}
          onChange={onGitRepoChange}
        />
        {/* a11y [1.3.1]: label associated with the branch input via htmlFor/id */}
        <div class={formStyles.fieldGroup}>
          <label htmlFor="settings-branch" class={formStyles.label}>Branch</label>
          <div class={styles.copyRow}>
            <input
              id="settings-branch"
              name="branch"
              class={formStyles.inputMono}
              list="settings-branch-options"
              value={gitBranch}
              onInput={(e) => onGitBranchChange((e.target as HTMLInputElement).value)}
            />
            <button
              type="button"
              class={styles.copyButton}
              onClick={loadBranches}
              disabled={loadingBranches}
              aria-label="Load branches from repository"
              title="Load branches from repository"
            >
              <GitBranch size={14} />
            </button>
          </div>
          <datalist id="settings-branch-options">
            {branches.map((b) => <option key={b} value={b} />)}
          </datalist>
          <span class={styles.fieldHint}>
            {branchError
              ? branchError
              : loadingBranches
                ? 'Loading branches…'
                : branches.length > 0
                  ? `${branches.length} branches loaded — type to filter`
                  : 'Click the icon to fetch branches from the repository.'}
          </span>
        </div>
        <Input
          label="Build Command"
          name="build-cmd"
          id="settings-build-cmd"
          mono
          value={buildCommand}
          onChange={onBuildCommandChange}
          placeholder="bun run build"
        />
        <div>
          <label htmlFor="settings-project" class={formStyles.label}>
            <FolderOpen size={14} style={{ verticalAlign: 'text-bottom', marginRight: '4px' }} />
            Project
          </label>
          {/* a11y [WCAG 4.1.2]: select has associated label via htmlFor/id */}
          <Select
            id="settings-project"
            options={[{ value: '', label: 'Unassigned' }, ...projects.map((p) => ({ value: p.id, label: p.name }))]}
            value={selectedProjectId}
            onChange={onProjectChange}
            fullWidth
          />
          <span class={styles.fieldHint}>Group this app with others in a project.</span>
        </div>
      </div>
    </div>
  );
}
