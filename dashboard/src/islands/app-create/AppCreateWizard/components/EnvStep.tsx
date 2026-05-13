import styles from '../app-create.module.css';
import formStyles from '@styles/form.module.css';

type DeploySource = 'git' | 'image' | 'compose';

type Props = {
  envContent: string;
  deploySource: DeploySource | null;
  onUpdate: (field: string, value: string) => void;
};

export default function EnvStep({ envContent, deploySource, onUpdate }: Props) {
  return (
    <div>
      <label htmlFor="create-env-vars" class={formStyles.label}>Environment Variables</label>
      <p class={styles.envDescription}>
        Paste your .env file content below. One KEY=value pair per line.
        {deploySource === 'compose' && (
          <> These will be available for {'${VAR}'} interpolation in your compose file.</>
        )}
      </p>
      <textarea
        id="create-env-vars"
        value={envContent}
        onInput={(e) => onUpdate('envContent', (e.target as HTMLTextAreaElement).value)}
        placeholder="DATABASE_URL=postgres://...&#10;API_KEY=secret123"
        rows={10}
        class={formStyles.textarea}
      />
    </div>
  );
}
