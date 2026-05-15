import { useState } from 'preact/hooks';
import { Copy, Check } from 'lucide-preact';
import styles from './code-block.module.css';

type Props = {
  code: string;
  language?: string;
  copyable?: boolean;
};

export default function CodeBlock({ code, language, copyable = true }: Props) {
  const [copied, setCopied] = useState(false);

  function handleCopy() {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div class={styles.wrapper}>
      {(language || copyable) && (
        <div class={styles.header}>
          {language && <span class={styles.language}>{language}</span>}
          {copyable && (
            <button type="button" class={styles.copyBtn} onClick={handleCopy} aria-label="Copy code">
              {copied ? <Check size={14} /> : <Copy size={14} />}
            </button>
          )}
        </div>
      )}
      <pre class={styles.pre}><code>{code}</code></pre>
    </div>
  );
}
