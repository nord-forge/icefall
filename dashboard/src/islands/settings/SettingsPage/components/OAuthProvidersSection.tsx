import { useState, useEffect } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import { Save, Key, CheckCircle, Copy } from 'lucide-preact';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  onSaveMessage: (msg: string) => void;
};

export default function OAuthProvidersSection({ onSaveMessage }: Props) {
  const [githubClientId, setGithubClientId] = useState('');
  const [githubClientSecret, setGithubClientSecret] = useState('');
  const [githubEnabled, setGithubEnabled] = useState(false);
  const [githubHasSecret, setGithubHasSecret] = useState(false);
  const [githubCallbackUrl, setGithubCallbackUrl] = useState('');
  const [googleClientId, setGoogleClientId] = useState('');
  const [googleClientSecret, setGoogleClientSecret] = useState('');
  const [googleEnabled, setGoogleEnabled] = useState(false);
  const [googleHasSecret, setGoogleHasSecret] = useState(false);
  const [googleCallbackUrl, setGoogleCallbackUrl] = useState('');
  const [saving, setSaving] = useState(false);
  const [copiedCallback, setCopiedCallback] = useState('');

  useEffect(() => {
    fetch('/api/v1/settings/oauth', { credentials: 'same-origin' }).then(r => r.json()).then(d => {
      if (d.data) {
        setGithubClientId(d.data.github_client_id || '');
        setGithubEnabled(d.data.github_enabled || false);
        setGithubHasSecret(d.data.github_has_secret || false);
        setGithubCallbackUrl(d.data.github_callback_url || '');
        setGoogleClientId(d.data.google_client_id || '');
        setGoogleEnabled(d.data.google_enabled || false);
        setGoogleHasSecret(d.data.google_has_secret || false);
        setGoogleCallbackUrl(d.data.google_callback_url || '');
      }
    }).catch(() => {});
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      const body: Record<string, any> = {
        github_client_id: githubClientId || undefined,
        github_enabled: githubEnabled,
        google_client_id: googleClientId || undefined,
        google_enabled: googleEnabled,
      };
      if (githubClientSecret) body.github_client_secret = githubClientSecret;
      if (googleClientSecret) body.google_client_secret = googleClientSecret;

      const res = await fetch('/api/v1/settings/oauth', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'same-origin',
        body: JSON.stringify(body),
      });
      const d = await res.json();
      if (d.data) {
        setGithubHasSecret(d.data.github_has_secret);
        setGoogleHasSecret(d.data.google_has_secret);
        setGithubClientSecret('');
        setGoogleClientSecret('');
      }
      onSaveMessage('OAuth settings saved');
    } catch { onSaveMessage('Failed to save OAuth settings'); }
    setSaving(false);
  }

  function copyCallback(url: string, provider: string) {
    navigator.clipboard.writeText(url).then(() => {
      setCopiedCallback(provider);
      setTimeout(() => setCopiedCallback(''), 2000);
    });
  }

  return (
    <div id="oauth" class={styles.section}>
      <h2 class={styles.sectionHeading}><Key size={18} aria-hidden="true" /> OAuth Providers</h2>
      <p class={styles.hint} style={{ marginTop: 0, marginBottom: 'var(--space-4)' }}>
        Allow users to sign in with GitHub or Google. Create an OAuth App with each provider and enter the credentials below.
      </p>

      {/* GitHub */}
      <div style={{ marginBottom: 'var(--space-5)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', marginBottom: 'var(--space-3)' }}>
          <h3 class={styles.subHeading} style={{ margin: 0 }}>GitHub</h3>
          <button
            type="button"
            role="switch"
            aria-checked={githubEnabled}
            aria-label="Enable GitHub OAuth"
            class={`${styles.toggle} ${githubEnabled ? styles.toggleOn : ''}`}
            onClick={() => setGithubEnabled(!githubEnabled)}
          >
            <span class={styles.toggleKnob}>
                {/* a11y [WCAG 1.4.1]: shape cue inside knob — not color alone */}
                <svg class={styles.toggleIcon} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
                  <path class={styles.toggleCheck} d="M2.5 5 L4.5 7 L7.5 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
                  <path class={styles.toggleCross} d="M3 3 L7 7 M7 3 L3 7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
                </svg>
              </span>
          </button>
        </div>
        <div class={formStyles.fieldRow}>
          <div>
            <label htmlFor="oauth-gh-id" class={formStyles.label}>Client ID</label>
            <input
              id="oauth-gh-id"
              class={formStyles.inputMono}
              value={githubClientId}
              onInput={e => setGithubClientId((e.target as HTMLInputElement).value)}
              placeholder="Iv1.abc123..."
            />
          </div>
          <div>
            <label htmlFor="oauth-gh-secret" class={formStyles.label}>
              Client Secret {githubHasSecret && <span style={{ fontSize: '0.75rem', color: 'var(--color-text-secondary)' }}>(configured)</span>}
            </label>
            <input
              id="oauth-gh-secret"
              class={formStyles.inputMono}
              type="password"
              autoComplete="off"
              value={githubClientSecret}
              onInput={e => setGithubClientSecret((e.target as HTMLInputElement).value)}
              placeholder={githubHasSecret ? 'Leave blank to keep current' : 'Enter client secret'}
            />
          </div>
        </div>
        {githubCallbackUrl && (
          <div style={{ marginTop: 'var(--space-2)' }}>
            <label class={formStyles.label}>Callback URL</label>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
              <code class={styles.codeInline}>{githubCallbackUrl}</code>
              <button
                type="button"
                class={styles.iconButton}
                onClick={() => copyCallback(githubCallbackUrl, 'github')}
                aria-label="Copy GitHub callback URL"
              >
                {copiedCallback === 'github' ? <CheckCircle size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
              </button>
            </div>
            <p class={formStyles.hint}>
              Add this URL as the callback in your <a href="https://github.com/settings/developers" target="_blank" rel="noopener noreferrer">GitHub OAuth App settings</a>.
            </p>
          </div>
        )}
      </div>

      {/* Google */}
      <div style={{ marginBottom: 'var(--space-3)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', marginBottom: 'var(--space-3)' }}>
          <h3 class={styles.subHeading} style={{ margin: 0 }}>Google</h3>
          <button
            type="button"
            role="switch"
            aria-checked={googleEnabled}
            aria-label="Enable Google OAuth"
            class={`${styles.toggle} ${googleEnabled ? styles.toggleOn : ''}`}
            onClick={() => setGoogleEnabled(!googleEnabled)}
          >
            <span class={styles.toggleKnob}>
                {/* a11y [WCAG 1.4.1]: shape cue inside knob — not color alone */}
                <svg class={styles.toggleIcon} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
                  <path class={styles.toggleCheck} d="M2.5 5 L4.5 7 L7.5 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
                  <path class={styles.toggleCross} d="M3 3 L7 7 M7 3 L3 7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
                </svg>
              </span>
          </button>
        </div>
        <div class={formStyles.fieldRow}>
          <div>
            <label htmlFor="oauth-gl-id" class={formStyles.label}>Client ID</label>
            <input
              id="oauth-gl-id"
              class={formStyles.inputMono}
              value={googleClientId}
              onInput={e => setGoogleClientId((e.target as HTMLInputElement).value)}
              placeholder="123456789.apps.googleusercontent.com"
            />
          </div>
          <div>
            <label htmlFor="oauth-gl-secret" class={formStyles.label}>
              Client Secret {googleHasSecret && <span style={{ fontSize: '0.75rem', color: 'var(--color-text-secondary)' }}>(configured)</span>}
            </label>
            <input
              id="oauth-gl-secret"
              class={formStyles.inputMono}
              type="password"
              autoComplete="off"
              value={googleClientSecret}
              onInput={e => setGoogleClientSecret((e.target as HTMLInputElement).value)}
              placeholder={googleHasSecret ? 'Leave blank to keep current' : 'Enter client secret'}
            />
          </div>
        </div>
        {googleCallbackUrl && (
          <div style={{ marginTop: 'var(--space-2)' }}>
            <label class={formStyles.label}>Callback URL</label>
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
              <code class={styles.codeInline}>{googleCallbackUrl}</code>
              <button
                type="button"
                class={styles.iconButton}
                onClick={() => copyCallback(googleCallbackUrl, 'google')}
                aria-label="Copy Google callback URL"
              >
                {copiedCallback === 'google' ? <CheckCircle size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
              </button>
            </div>
            <p class={formStyles.hint}>
              Add this URL as an authorized redirect URI in the <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer">Google Cloud Console</a>.
            </p>
          </div>
        )}
      </div>

      <div class={styles.saveRow}>
        <Button variant="primary" onClick={handleSave} loading={saving}>
          <Save size={14} aria-hidden="true" /> Save OAuth Settings
        </Button>
      </div>
    </div>
  );
}
