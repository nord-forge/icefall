import Input from '@islands/shared/Input/Input';
import Textarea from '@islands/shared/Textarea/Textarea';
import styles from '../app-create.module.css';
import formStyles from '@styles/form.module.css';

/** Parse service names from a compose YAML string (client-side preview). */
export function parseComposeServices(yaml: string): string[] {
  try {
    // Simple YAML service name extraction — look for top-level keys under "services:"
    const lines = yaml.split('\n');
    let inServices = false;
    const names: string[] = [];
    for (const line of lines) {
      if (/^services:\s*$/.test(line)) {
        inServices = true;
        continue;
      }
      if (inServices) {
        // A service name is indented exactly 2 spaces (standard compose indent)
        const match = line.match(/^  ([a-zA-Z0-9_-]+):\s*$/);
        if (match) {
          names.push(match[1]);
        }
        // Stop when we hit another top-level key
        if (/^[a-zA-Z]/.test(line) && !line.startsWith(' ')) {
          inServices = false;
        }
      }
    }
    return names;
  } catch {
    return [];
  }
}

type Props = {
  name: string;
  composeContent: string;
  composeError: string;
  validationErrors: Record<string, string>;
  onUpdate: (field: string, value: string) => void;
};

export default function ComposeStep({
  name,
  composeContent,
  composeError,
  validationErrors,
  onUpdate,
}: Props) {
  const services = parseComposeServices(composeContent);
  return (
    <div class={formStyles.fieldGroup}>
      <Input
        label="Stack Name"
        name="compose-name"
        id="create-compose-name"
        value={name}
        onChange={(v) => onUpdate('name', v)}
        placeholder="my-wordpress-stack"
        error={validationErrors.name}
      />
      <div>
        <Textarea
          label="Compose File"
          name="compose-content"
          id="create-compose-content"
          value={composeContent}
          onChange={(v) => onUpdate('compose_content', v)}
          placeholder={`services:\n  web:\n    image: nginx:latest\n    ports:\n      - "80:80"\n  db:\n    image: postgres:16\n    environment:\n      POSTGRES_PASSWORD: secret`}
          rows={14}
          mono
          helpText="Paste your docker-compose.yml content. Only pre-built images are supported (no build directive)."
          error={validationErrors.compose_content}
        />
        {composeError && (
          <p role="alert" class={styles.composeError}>{composeError}</p>
        )}
      </div>
      {services.length > 0 && (
        <div class={styles.servicePreview}>
          <span class={styles.servicePreviewLabel}>
            {services.length} service{services.length !== 1 ? 's' : ''} detected
          </span>
          <ul class={styles.serviceList} aria-label="Detected services">
            {services.map((s) => (
              <li key={s} class={styles.serviceItem}>{s}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
