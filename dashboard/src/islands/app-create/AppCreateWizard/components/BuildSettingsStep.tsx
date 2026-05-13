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
      <div>
        <label htmlFor="create-build-cmd" class={formStyles.label}>Build Command</label>
        <input id="create-build-cmd" class={formStyles.inputMono} value={buildCommand} onInput={(e) => onUpdate('build_command', (e.target as HTMLInputElement).value)} placeholder="bun run build" />
      </div>
      <div>
        <label htmlFor="create-output-dir" class={formStyles.label}>Output Directory</label>
        <input id="create-output-dir" class={formStyles.inputMono} value={outputDir} onInput={(e) => onUpdate('output_dir', (e.target as HTMLInputElement).value)} placeholder="dist" />
      </div>
      <div>
        <label htmlFor="create-start-cmd" class={formStyles.label}>Start Command</label>
        <input id="create-start-cmd" class={formStyles.inputMono} value={startCommand} onInput={(e) => onUpdate('start_command', (e.target as HTMLInputElement).value)} placeholder="node server.js" />
      </div>
      <div>
        <label htmlFor="create-port" class={formStyles.label}>Port</label>
        <input id="create-port" class={formStyles.inputMono} value={port} onInput={(e) => onUpdate('port', (e.target as HTMLInputElement).value)} />
      </div>
    </div>
  );
}
