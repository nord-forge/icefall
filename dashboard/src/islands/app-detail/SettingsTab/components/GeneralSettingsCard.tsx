import type { Project } from '@lib/types';
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
        <div>
          <label htmlFor="settings-app-name" class={formStyles.label}>App Name</label>
          <input id="settings-app-name" class={formStyles.input} value={name} onInput={(e) => onNameChange((e.target as HTMLInputElement).value)} />
        </div>
        <div>
          <label htmlFor="settings-git-repo" class={formStyles.label}>Git Repository</label>
          <input id="settings-git-repo" class={formStyles.inputMono} value={gitRepo} onInput={(e) => onGitRepoChange((e.target as HTMLInputElement).value)} />
        </div>
        <div>
          <label htmlFor="settings-branch" class={formStyles.label}>Branch</label>
          <input id="settings-branch" class={formStyles.inputMono} value={gitBranch} onInput={(e) => onGitBranchChange((e.target as HTMLInputElement).value)} />
        </div>
        <div>
          <label htmlFor="settings-build-cmd" class={formStyles.label}>Build Command</label>
          <input id="settings-build-cmd" class={formStyles.inputMono} value={buildCommand} onInput={(e) => onBuildCommandChange((e.target as HTMLInputElement).value)} placeholder="bun run build" />
        </div>
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
