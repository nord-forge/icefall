import Textarea from '@islands/shared/Textarea/Textarea';
import styles from '../app-create.module.css';

type DeploySource = 'git' | 'image' | 'compose';

type Props = {
  envContent: string;
  deploySource: DeploySource | null;
  onUpdate: (field: string, value: string) => void;
};

export default function EnvStep({ envContent, deploySource, onUpdate }: Props) {
  return (
    <div>
      <p class={styles.envDescription}>
        Paste your .env file content below. One KEY=value pair per line.
        {deploySource === 'compose' && (
          <> These will be available for {'${VAR}'} interpolation in your compose file.</>
        )}
      </p>
      <Textarea
        label="Environment Variables"
        name="env-vars"
        id="create-env-vars"
        value={envContent}
        onChange={(v) => onUpdate('envContent', v)}
        placeholder="DATABASE_URL=postgres://...&#10;API_KEY=secret123"
        rows={10}
        mono
      />
    </div>
  );
}
