import { useState, useEffect, useRef } from 'preact/hooks';
import QRCode from 'qrcode';
import { api } from '@lib/api';
import Button from '@islands/shared/Button/Button';
import { Shield, Copy } from 'lucide-preact';
import styles from './two-factor-section.module.css';
import formStyles from '@styles/form.module.css';

type SetupStep = 'idle' | 'qr' | 'backup' | 'done';
type ManageAction = 'none' | 'disable' | 'regenerate';

export default function TwoFactorSection() {
  const [totpEnabled, setTotpEnabled] = useState(false);
  const [loading, setLoading] = useState(true);
  const successTimeoutRef = useRef<number>();

  // Setup flow state
  const [setupStep, setSetupStep] = useState<SetupStep>('idle');
  const [otpauthUrl, setOtpauthUrl] = useState('');
  const [secret, setSecret] = useState('');
  const qrCanvasRef = useRef<HTMLCanvasElement>(null);
  const [setupCode, setSetupCode] = useState('');
  const [backupCodes, setBackupCodes] = useState<string[]>([]);
  const [backupsSaved, setBackupsSaved] = useState(false);

  // Management state
  const [manageAction, setManageAction] = useState<ManageAction>('none');
  const [manageCode, setManageCode] = useState('');

  // Feedback
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    api.getMe()
      .then(({ data }) => {
        setTotpEnabled(data.totp_enabled ?? false);
      })
      .catch(() => {})
      .finally(() => setLoading(false));

    return () => clearTimeout(successTimeoutRef.current);
  }, []);

  // Render the QR client-side from the otpauth URL onto a canvas — avoids
  // injecting server-supplied SVG markup via dangerouslySetInnerHTML.
  useEffect(() => {
    if (setupStep !== 'qr' || !otpauthUrl || !qrCanvasRef.current) return;
    QRCode.toCanvas(qrCanvasRef.current, otpauthUrl, { width: 200, margin: 1 })
      .catch(() => setError('Failed to render QR code; use the manual key below.'));
  }, [setupStep, otpauthUrl]);

  async function startSetup() {
    setError('');
    setSubmitting(true);
    try {
      const { data } = await api.setup2fa();
      setOtpauthUrl(data.otpauth_url);
      setSecret(data.secret);
      setSetupStep('qr');
    } catch (e: any) {
      setError(e.message || 'Failed to start 2FA setup');
    }
    setSubmitting(false);
  }

  async function confirmSetup() {
    if (!setupCode.trim()) {
      setError('Enter the 6-digit code from your authenticator app');
      return;
    }
    setError('');
    setSubmitting(true);
    try {
      const { data } = await api.verify2fa(setupCode.trim());
      setBackupCodes(data.backup_codes);
      setTotpEnabled(true);
      setSetupStep('backup');
    } catch (e: any) {
      setError(e.message || 'Invalid code');
    }
    setSubmitting(false);
  }

  function finishSetup() {
    setSetupStep('done');
    setOtpauthUrl('');
    setSecret('');
    setSetupCode('');
    setBackupsSaved(false);
    setSuccess('Two-factor authentication is now active.');
    clearTimeout(successTimeoutRef.current);
    successTimeoutRef.current = window.setTimeout(() => setSuccess(''), 5000);
  }

  async function handleDisable() {
    if (!manageCode.trim()) {
      setError('Enter your current TOTP code or a backup code');
      return;
    }
    setError('');
    setSubmitting(true);
    try {
      await api.disable2fa(manageCode.trim());
      setTotpEnabled(false);
      setManageAction('none');
      setManageCode('');
      setSuccess('Two-factor authentication has been disabled.');
      clearTimeout(successTimeoutRef.current);
      successTimeoutRef.current = window.setTimeout(() => setSuccess(''), 5000);
    } catch (e: any) {
      setError(e.message || 'Invalid code');
    }
    setSubmitting(false);
  }

  async function handleRegenerate() {
    if (!manageCode.trim()) {
      setError('Enter your current TOTP code');
      return;
    }
    setError('');
    setSubmitting(true);
    try {
      const { data } = await api.regenerateBackupCodes(manageCode.trim());
      setBackupCodes(data.backup_codes);
      setManageAction('none');
      setManageCode('');
      setSetupStep('backup');
    } catch (e: any) {
      setError(e.message || 'Invalid code');
    }
    setSubmitting(false);
  }

  function cancelAction() {
    setManageAction('none');
    setManageCode('');
    setError('');
    if (setupStep === 'qr') {
      setSetupStep('idle');
      setQrSvg('');
      setSecret('');
      setSetupCode('');
    }
  }

  function copyBackupCodes() {
    const text = backupCodes.join('\n');
    navigator.clipboard.writeText(text).catch(() => {});
  }

  if (loading) return null;

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}>
          {/* a11y [1.1.1]: decorative icon hidden from assistive tech */}
          <Shield size={18} aria-hidden="true" /> Two-Factor Authentication
        </h2>
        <span class={totpEnabled ? styles.statusEnabled : styles.statusDisabled}>
          {totpEnabled ? 'Enabled' : 'Disabled'}
        </span>
      </div>

      <p class={styles.description}>
        Add an extra layer of security to your account. When enabled, you will need to enter
        a code from your authenticator app each time you sign in.
      </p>

      {error && <p class={styles.errorText} role="alert">{error}</p>}
      {success && <p class={styles.successText} role="status">{success}</p>}

      {/* --- Not enabled: show setup flow --- */}
      {!totpEnabled && setupStep === 'idle' && (
        <Button variant="primary" onClick={startSetup} loading={submitting}>
          Enable Two-Factor Authentication
        </Button>
      )}

      {!totpEnabled && setupStep === 'qr' && (
        <div class={styles.stepContainer}>
          <p class={styles.description} style={{ marginBottom: 0 }}>
            Scan this QR code with your authenticator app (Google Authenticator, Authy, 1Password, etc.),
            then enter the 6-digit code below to confirm.
          </p>

          <div class={styles.qrWrapper}>
            {/* a11y [WCAG 1.1.1 + 4.1.2]: canvas is opaque to AT — role=img + aria-label give it a name and a text alternative */}
            <canvas
              ref={qrCanvasRef}
              class={styles.qrSvg}
              role="img"
              aria-label="QR code for setting up two-factor authentication. Scan with your authenticator app."
            />
            <span class={styles.secretLabel}>Or enter this key manually:</span>
            <code class={styles.secretKey}>{secret}</code>
          </div>

          <div>
            {/* a11y [1.3.1]: label associated with input */}
            <label for="setup-totp-code" style={{ display: 'block', fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-medium)', marginBottom: '4px' }}>
              Verification code
            </label>
            <input
              id="setup-totp-code"
              class={styles.codeInput}
              type="text"
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={6}
              placeholder="000000"
              value={setupCode}
              onInput={e => setSetupCode((e.target as HTMLInputElement).value)}
              onKeyDown={e => { if (e.key === 'Enter') confirmSetup(); }}
            />
          </div>

          <div class={styles.actionRow}>
            <Button variant="primary" onClick={confirmSetup} loading={submitting} disabled={setupCode.length < 6}>
              Verify and Enable
            </Button>
            <Button variant="ghost" onClick={cancelAction}>Cancel</Button>
          </div>
        </div>
      )}

      {/* --- Backup codes display (after setup or regeneration) --- */}
      {setupStep === 'backup' && backupCodes.length > 0 && (
        <div class={styles.stepContainer}>
          <p class={styles.description} style={{ marginBottom: 0 }}>
            Save these backup codes in a secure location. Each code can only be used once.
            If you lose access to your authenticator app, use a backup code to sign in.
          </p>

          <div class={styles.backupCodesGrid} aria-label="Backup codes">
            {backupCodes.map((code, i) => (
              <div key={i} class={styles.backupCode}>{code}</div>
            ))}
          </div>

          <button
            type="button"
            onClick={copyBackupCodes}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              fontSize: 'var(--text-sm)',
              color: 'var(--color-text-secondary)',
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              padding: 0,
              fontFamily: 'inherit',
            }}
            aria-label="Copy all backup codes to clipboard"
          >
            <Copy size={14} aria-hidden="true" /> Copy all codes
          </button>

          <p class={styles.backupWarning}>
            These codes will not be shown again.
          </p>

          <div class={styles.confirmRow}>
            <input
              type="checkbox"
              class={formStyles.checkbox}
              id="backup-saved-check"
              checked={backupsSaved}
              onChange={() => setBackupsSaved(!backupsSaved)}
            />
            <label for="backup-saved-check">I have saved my backup codes</label>
          </div>

          <Button variant="primary" onClick={finishSetup} disabled={!backupsSaved}>
            Done
          </Button>
        </div>
      )}

      {/* --- Enabled: management options --- */}
      {totpEnabled && setupStep !== 'backup' && manageAction === 'none' && (
        <div class={styles.actionRow}>
          <Button variant="secondary" onClick={() => { setManageAction('regenerate'); setError(''); }}>
            Regenerate Backup Codes
          </Button>
          <Button variant="ghost" onClick={() => { setManageAction('disable'); setError(''); }}>
            Disable 2FA
          </Button>
        </div>
      )}

      {/* --- Disable 2FA prompt --- */}
      {totpEnabled && manageAction === 'disable' && (
        <div class={styles.stepContainer}>
          <p class={styles.description} style={{ marginBottom: 0 }}>
            Enter your current TOTP code or a backup code to disable two-factor authentication.
          </p>
          <div>
            <label for="disable-totp-code" style={{ display: 'block', fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-medium)', marginBottom: '4px' }}>
              Authentication code
            </label>
            <input
              id="disable-totp-code"
              class={styles.codeInput}
              type="text"
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={8}
              placeholder="000000"
              value={manageCode}
              onInput={e => setManageCode((e.target as HTMLInputElement).value)}
              onKeyDown={e => { if (e.key === 'Enter') handleDisable(); }}
            />
          </div>
          <div class={styles.actionRow}>
            <Button variant="primary" onClick={handleDisable} loading={submitting} disabled={manageCode.length < 6}>
              Disable 2FA
            </Button>
            <Button variant="ghost" onClick={cancelAction}>Cancel</Button>
          </div>
        </div>
      )}

      {/* --- Regenerate backup codes prompt --- */}
      {totpEnabled && manageAction === 'regenerate' && (
        <div class={styles.stepContainer}>
          <p class={styles.description} style={{ marginBottom: 0 }}>
            Enter your current TOTP code to generate new backup codes. This will invalidate all existing backup codes.
          </p>
          <div>
            <label for="regen-totp-code" style={{ display: 'block', fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-medium)', marginBottom: '4px' }}>
              Authentication code
            </label>
            <input
              id="regen-totp-code"
              class={styles.codeInput}
              type="text"
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={6}
              placeholder="000000"
              value={manageCode}
              onInput={e => setManageCode((e.target as HTMLInputElement).value)}
              onKeyDown={e => { if (e.key === 'Enter') handleRegenerate(); }}
            />
          </div>
          <div class={styles.actionRow}>
            <Button variant="primary" onClick={handleRegenerate} loading={submitting} disabled={manageCode.length < 6}>
              Regenerate Codes
            </Button>
            <Button variant="ghost" onClick={cancelAction}>Cancel</Button>
          </div>
        </div>
      )}
    </div>
  );
}
