import styles from '../app-create.module.css';
import formStyles from '@styles/form.module.css';

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
  return (
    <div class={formStyles.fieldGroup}>
      <div>
        <label htmlFor="create-app-name" class={formStyles.label}>App Name</label>
        <input id="create-app-name" class={formStyles.input} value={name} onInput={(e) => onUpdate('name', (e.target as HTMLInputElement).value)} placeholder="my-awesome-app" aria-invalid={!!validationErrors.name} aria-describedby={validationErrors.name ? 'err-name' : undefined} />
        {validationErrors.name && <p id="err-name" role="alert" class={styles.fieldError}>{validationErrors.name}</p>}
      </div>
      <div>
        <label htmlFor="create-repo-url" class={formStyles.label}>Repository URL</label>
        <input id="create-repo-url" class={formStyles.inputMono} value={gitRepo} onInput={(e) => onUpdate('git_repo', (e.target as HTMLInputElement).value)} placeholder="https://github.com/user/repo" />
      </div>
      <div>
        <label htmlFor="create-branch" class={formStyles.label}>Branch</label>
        <input id="create-branch" class={formStyles.inputMono} value={gitBranch} onInput={(e) => onUpdate('git_branch', (e.target as HTMLInputElement).value)} />
      </div>
    </div>
  );
}
