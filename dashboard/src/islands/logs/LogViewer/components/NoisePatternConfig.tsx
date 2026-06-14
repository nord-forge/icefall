import { useState } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import { Save } from 'lucide-preact';
import Textarea from '@islands/shared/Textarea/Textarea';
import styles from './noise-pattern-config.module.css';

type Props = {
  appId: string;
  noisePatterns: string;
  highlightPatterns: string;
};

export default function NoisePatternConfig({ appId, noisePatterns, highlightPatterns }: Props) {
  const [noise, setNoise] = useState(noisePatterns);
  const [highlight, setHighlight] = useState(highlightPatterns);
  const [saving, setSaving] = useState(false);

  async function handleSave() {
    setSaving(true);
    try {
      await api.updateApp(appId, {
        log_noise_patterns: noise,
        log_highlight_patterns: highlight,
      });
      addToast('success', 'Log patterns saved');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to save log patterns');
    }
    setSaving(false);
  }

  return (
    <div class={styles.container}>
      <div class={styles.patternGroup}>
        <Textarea
          label="Noise patterns (hidden lines)"
          name="noise-patterns"
          id="noise-patterns"
          className={styles.patternTextarea}
          value={noise}
          onChange={setNoise}
          placeholder={"health_check\\nkeepalive\\nGET /favicon.ico"}
          helpText="One pattern per line. Lines matching any pattern will be hidden in smart mode. Patterns are matched as regular expressions, or as plain text if the regex is invalid."
        />
      </div>

      <div class={styles.patternGroup}>
        <Textarea
          label="Highlight patterns (emphasized lines)"
          name="highlight-patterns"
          id="highlight-patterns"
          className={styles.patternTextarea}
          value={highlight}
          onChange={setHighlight}
          placeholder={"ERROR\\nWARN\\nCRITICAL"}
          helpText="One pattern per line. Lines matching any pattern will be visually emphasized. Patterns are matched as regular expressions, or as plain text if the regex is invalid."
        />
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button variant="primary" onClick={handleSave} loading={saving}>
          <Save size={14} aria-hidden="true" /> Save Patterns
        </Button>
      </div>
    </div>
  );
}
