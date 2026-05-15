import Input from '@islands/shared/Input/Input';
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
      <Input
        label="Branch"
        name="branch"
        id="create-branch"
        mono
        value={gitBranch}
        onChange={(v) => onUpdate('git_branch', v)}
      />
    </div>
  );
}
