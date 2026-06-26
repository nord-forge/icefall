import { useState, useEffect, useRef, useCallback } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import { Copy, CheckCircle, ExternalLink, ShieldCheck } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import styles from './oauth-setup-dialog.module.css';
import formStyles from '@styles/form.module.css';

export type OAuthProvider = 'github' | 'google';

type ProviderCopy = {
  name: string;
  idLabel: string;
  idPlaceholder: string;
  consoleUrl: string;
  consoleName: string;
  callbackHint: string;
  steps: string[];
};

const COPY: Record<OAuthProvider, ProviderCopy> = {
  github: {
    name: 'GitHub',
    idLabel: 'Client ID',
    idPlaceholder: 'Iv1.abc123...',
    consoleUrl: 'https://github.com/settings/developers',
    consoleName: 'GitHub Developer settings',
    callbackHint: 'Paste this as the Authorization callback URL when creating the app.',
    steps: [
      'Open GitHub Developer settings and click "New OAuth App".',
      'Set the Authorization callback URL to the value below.',
      'Create the app, then copy its Client ID and generate a Client secret.',
      'Paste both here, save, then run Test connection.',
    ],
  },
  google: {
    name: 'Google',
    idLabel: 'Client ID',
    idPlaceholder: '123456789.apps.googleusercontent.com',
    consoleUrl: 'https://console.cloud.google.com/apis/credentials',
    consoleName: 'Google Cloud Console',
    callbackHint: 'Add this as an Authorized redirect URI on the OAuth client.',
    steps: [
      'In Google Cloud Console, create an OAuth client ID (type: Web application).',
      'Add the value below as an Authorized redirect URI.',
      'Copy the resulting Client ID and Client secret.',
      'Paste both here, save, then run Test connection.',
    ],
  },
};

type Props = {
  provider: OAuthProvider;
  open: boolean;
  /** Existing client id (for edit mode); empty when first configuring. */
  initialClientId: string;
  hasSecret: boolean;
  callbackUrl: string;
  onClose: () => void;
  /** Called after a successful save so the parent can refresh status. */
  onSaved: () => void;
};

export default function OAuthSetupDialog({
  provider, open, initialClientId, hasSecret, callbackUrl, onClose, onSaved,
}: Props) {
  const copy = COPY[provider];
  const [clientId, setClientId] = useState(initialClientId);
  const [clientSecret, setClientSecret] = useState('');
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [savedOnce, setSavedOnce] = useState(hasSecret && !!initialClientId);
  const [copied, setCopied] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    setClientId(initialClientId);
    setClientSecret('');
    setSavedOnce(hasSecret && !!initialClientId);
  }, [initialClientId, hasSecret, open]);

  useEffect(() => {
    if (open) {
      previousFocusRef.current = document.activeElement as HTMLElement;
      requestAnimationFrame(() => dialogRef.current?.querySelector('input')?.focus());
      document.body.style.overflow = 'hidden';
    } else if (previousFocusRef.current) {
      previousFocusRef.current.focus();
      previousFocusRef.current = null;
    }
    return () => { document.body.style.overflow = ''; };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') { e.preventDefault(); onClose(); }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  const trapFocus = useCallback((e: KeyboardEvent) => {
    if (e.key !== 'Tab' || !dialogRef.current) return;
    const f = dialogRef.current.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), [tabindex]:not([tabindex="-1"])'
    );
    if (!f.length) return;
    const first = f[0], last = f[f.length - 1];
    if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
  }, []);

  function copyCallback() {
    navigator.clipboard.writeText(callbackUrl).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  // Enabling on save means the credentials are live; that's required for the
  // round-trip test (the /authorize flow only runs for enabled providers).
  async function handleSave() {
    if (!clientId.trim() || (!clientSecret.trim() && !hasSecret)) return;
    setSaving(true);
    try {
      const body: Parameters<typeof api.updateOAuthSettings>[0] =
        provider === 'github'
          ? { github_client_id: clientId.trim(), github_enabled: true }
          : { google_client_id: clientId.trim(), google_enabled: true };
      if (clientSecret.trim()) {
        if (provider === 'github') body.github_client_secret = clientSecret.trim();
        else body.google_client_secret = clientSecret.trim();
      }
      await api.updateOAuthSettings(body);
      setClientSecret('');
      setSavedOnce(true);
      addToast('success', `${copy.name} credentials saved.`);
      onSaved();
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : `Failed to save ${copy.name} credentials.`);
    } finally {
      setSaving(false);
    }
  }

  // Real round-trip: open the provider consent in a popup via the /link flow,
  // which exchanges the code and lands on /profile?linked=success|error. We poll
  // the popup for that result, so the test exercises client id, secret AND the
  // callback URL end-to-end.
  function handleTest() {
    setTesting(true);
    const popup = window.open(
      `/api/v1/auth/oauth/${provider}/link`,
      'oauth-test',
      'width=600,height=720',
    );
    if (!popup) {
      setTesting(false);
      addToast('error', 'Popup blocked. Allow popups for this site and try again.');
      return;
    }
    const timer = setInterval(() => {
      try {
        if (popup.closed) {
          clearInterval(timer);
          setTesting(false);
          return;
        }
        const href = popup.location.href;
        if (href.includes('linked=success') || href.includes('linked=already')) {
          clearInterval(timer);
          popup.close();
          setTesting(false);
          addToast('success', `${copy.name} connection works — callback verified.`);
          onSaved();
        } else if (href.includes('error=')) {
          clearInterval(timer);
          popup.close();
          setTesting(false);
          const reason = new URL(href).searchParams.get('error') || 'unknown error';
          addToast('error', `${copy.name} test failed: ${reason.replace(/_/g, ' ')}.`);
        }
      } catch {
        // Cross-origin while on the provider's domain — keep polling.
      }
    }, 500);
  }

  if (!open) return null;
  const titleId = `oauth-setup-${provider}`;
  const canSave = clientId.trim() && (clientSecret.trim() || hasSecret);

  return (
    <div class={styles.backdrop} onClick={onClose}>
      {/* a11y [WCAG 4.1.2]: modal dialog with label */}
      <div
        ref={dialogRef}
        class={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={trapFocus}
      >
        <h2 id={titleId} class={styles.title}>Connect {copy.name}</h2>

        <ol class={styles.steps}>
          {copy.steps.map((s, i) => <li key={i}>{s}</li>)}
        </ol>

        <a class={styles.consoleLink} href={copy.consoleUrl} target="_blank" rel="noopener noreferrer">
          Open {copy.consoleName} <ExternalLink size={13} aria-hidden="true" />
        </a>

        {/* Callback URL — needed in step 2, so surfaced before the credential inputs. */}
        <div class={styles.callbackBlock}>
          <label class={formStyles.fieldLabel}>Callback URL</label>
          <div class={styles.callbackRow}>
            <code class={styles.callbackCode}>{callbackUrl}</code>
            <button
              type="button"
              class={styles.copyBtn}
              onClick={copyCallback}
              aria-label="Copy callback URL"
            >
              {copied ? <CheckCircle size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
            </button>
          </div>
          <p class={formStyles.helpText}>{copy.callbackHint}</p>
        </div>

        <div class={styles.fields}>
          <Input
            label={copy.idLabel}
            name={`oauth-${provider}-id`}
            id={`oauth-${provider}-id`}
            mono
            value={clientId}
            onChange={setClientId}
            placeholder={copy.idPlaceholder}
          />
          <Input
            label={`Client Secret${hasSecret ? ' (saved)' : ''}`}
            name={`oauth-${provider}-secret`}
            id={`oauth-${provider}-secret`}
            type="password"
            mono
            value={clientSecret}
            onChange={setClientSecret}
            placeholder={hasSecret ? 'Leave blank to keep current' : 'Enter client secret'}
          />
        </div>

        <div class={styles.actions}>
          <Button variant="ghost" onClick={onClose} disabled={saving || testing}>Close</Button>
          <Button
            variant="secondary"
            onClick={handleTest}
            loading={testing}
            disabled={!savedOnce || saving}
            title={savedOnce ? undefined : 'Save credentials first'}
          >
            <ShieldCheck size={14} aria-hidden="true" /> Test connection
          </Button>
          <Button variant="primary" onClick={handleSave} loading={saving} disabled={!canSave}>
            Save &amp; enable
          </Button>
        </div>
      </div>
    </div>
  );
}
