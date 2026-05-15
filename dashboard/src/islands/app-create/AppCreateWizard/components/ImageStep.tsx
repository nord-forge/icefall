import Input from '@islands/shared/Input/Input';
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
      <Input
        label="App Name"
        name="app-name"
        id="create-app-name"
        value={name}
        onChange={(v) => onUpdate('name', v)}
        placeholder="my-ghost-blog"
        error={validationErrors.name}
      />
      <Input
        label="Docker Image"
        name="image-ref"
        id="create-image-ref"
        mono
        value={imageRef}
        onChange={(v) => onUpdate('image_ref', v)}
        placeholder="ghost:5-alpine"
        error={validationErrors.image_ref}
        helpText={!validationErrors.image_ref ? 'Image name from Docker Hub or a full registry URL.' : undefined}
      />
      <Input
        label="Container Port"
        name="image-port"
        id="create-image-port"
        mono
        type="number"
        min={1}
        max={65535}
        value={port}
        onChange={(v) => onUpdate('port', v)}
        error={validationErrors.port}
        helpText={!validationErrors.port ? 'The port the container listens on internally.' : undefined}
      />
    </div>
  );
}
