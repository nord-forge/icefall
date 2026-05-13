import styles from '../app-create.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  name: string;
  imageRef: string;
  port: string;
  validationErrors: Record<string, string>;
  onUpdate: (field: string, value: string) => void;
};

export default function ImageStep({
  name,
  imageRef,
  port,
  validationErrors,
  onUpdate,
}: Props) {
  return (
    <div class={formStyles.fieldGroup}>
      <div>
        <label htmlFor="create-app-name" class={formStyles.label}>App Name</label>
        <input id="create-app-name" class={formStyles.input} value={name} onInput={(e) => onUpdate('name', (e.target as HTMLInputElement).value)} placeholder="my-ghost-blog" aria-invalid={!!validationErrors.name} aria-describedby={validationErrors.name ? 'err-name' : undefined} />
        {validationErrors.name && <p id="err-name" role="alert" class={styles.fieldError}>{validationErrors.name}</p>}
      </div>
      <div>
        <label htmlFor="create-image-ref" class={formStyles.label}>Docker Image</label>
        <input
          id="create-image-ref"
          class={formStyles.inputMono}
          value={imageRef}
          onInput={(e) => onUpdate('image_ref', (e.target as HTMLInputElement).value)}
          placeholder="ghost:5-alpine"
          aria-invalid={!!validationErrors.image_ref}
          aria-describedby={validationErrors.image_ref ? 'err-image-ref' : 'hint-image-ref'}
        />
        {validationErrors.image_ref ? (
          <p id="err-image-ref" role="alert" class={styles.fieldError}>{validationErrors.image_ref}</p>
        ) : (
          <span id="hint-image-ref" class={formStyles.hint}>
            Image name from Docker Hub or a full registry URL.
          </span>
        )}
      </div>
      <div>
        <label htmlFor="create-image-port" class={formStyles.label}>Container Port</label>
        <input
          id="create-image-port"
          class={formStyles.inputMono}
          type="number"
          min="1"
          max="65535"
          value={port}
          onInput={(e) => onUpdate('port', (e.target as HTMLInputElement).value)}
          aria-invalid={!!validationErrors.port}
          aria-describedby={validationErrors.port ? 'err-port' : 'hint-port'}
        />
        {validationErrors.port ? (
          <p id="err-port" role="alert" class={styles.fieldError}>{validationErrors.port}</p>
        ) : (
          <span id="hint-port" class={formStyles.hint}>
            The port the container listens on internally.
          </span>
        )}
      </div>
    </div>
  );
}
