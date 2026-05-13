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
      <div>
        <label htmlFor="create-compose-name" class={formStyles.label}>Stack Name</label>
        <input
          id="create-compose-name"
          class={formStyles.input}
          value={name}
          onInput={(e) => onUpdate('name', (e.target as HTMLInputElement).value)}
          placeholder="my-wordpress-stack"
          aria-invalid={!!validationErrors.name}
          aria-describedby={validationErrors.name ? 'err-name' : undefined}
        />
        {validationErrors.name && <p id="err-name" role="alert" class={styles.fieldError}>{validationErrors.name}</p>}
      </div>
      <div>
        <label htmlFor="create-compose-content" class={formStyles.label}>
          Compose File
        </label>
        <span class={formStyles.hint}>
          Paste your docker-compose.yml content. Only pre-built images are supported (no build directive).
        </span>
        <textarea
          id="create-compose-content"
          class={formStyles.textarea}
          value={composeContent}
          onInput={(e) => onUpdate('compose_content', (e.target as HTMLTextAreaElement).value)}
          placeholder={`services:\n  web:\n    image: nginx:latest\n    ports:\n      - "80:80"\n  db:\n    image: postgres:16\n    environment:\n      POSTGRES_PASSWORD: secret`}
          rows={14}
          spellcheck={false}
          style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}
          aria-invalid={!!validationErrors.compose_content}
          aria-describedby={validationErrors.compose_content ? 'err-compose' : undefined}
        />
        {validationErrors.compose_content && (
          <p id="err-compose" role="alert" class={styles.fieldError}>{validationErrors.compose_content}</p>
        )}
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
