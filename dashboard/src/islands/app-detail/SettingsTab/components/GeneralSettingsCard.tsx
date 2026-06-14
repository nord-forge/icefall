import type { Project } from '@lib/types';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import { FolderOpen } from 'lucide-preact';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
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
        <Input
          label="Branch"
          name="branch"
          id="settings-branch"
          mono
          value={gitBranch}
          onChange={onGitBranchChange}
        />
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
