import Input from '@islands/shared/Input/Input';
import formStyles from '@styles/form.module.css';

type Props = {
  buildCommand: string;
  outputDir: string;
  startCommand: string;
  port: string;
  onUpdate: (field: string, value: string) => void;
};

export default function BuildSettingsStep({
  buildCommand,
  outputDir,
  startCommand,
  port,
  onUpdate,
}: Props) {
  return (
    <div class={formStyles.fieldGroup}>
      <Input
        label="Build Command"
        name="build-cmd"
        id="create-build-cmd"
        mono
        value={buildCommand}
        onChange={(v) => onUpdate('build_command', v)}
        placeholder="bun run build"
      />
      <Input
        label="Output Directory"
        name="output-dir"
        id="create-output-dir"
        mono
        value={outputDir}
        onChange={(v) => onUpdate('output_dir', v)}
        placeholder="dist"
      />
      <Input
        label="Start Command"
        name="start-cmd"
        id="create-start-cmd"
        mono
        value={startCommand}
        onChange={(v) => onUpdate('start_command', v)}
        placeholder="node server.js"
      />
      <Input
        label="Port"
        name="port"
        id="create-port"
        mono
        value={port}
        onChange={(v) => onUpdate('port', v)}
      />
    </div>
  );
}
