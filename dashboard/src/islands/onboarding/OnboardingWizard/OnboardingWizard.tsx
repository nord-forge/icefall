import { useEffect, useState } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import { User, Server, Globe, GitBranch, Rocket, Check, ArrowRight } from 'lucide-preact';
import Input from '@islands/shared/Input/Input';
import { api, ApiError } from '@lib/api';
import styles from './onboarding.module.css';
import formStyles from '@styles/form.module.css';

type OnboardingStatus = {
  current_step: string;
  completed_steps: string[];
  is_complete: boolean;
}

const STEPS = [
  { key: 'admin_account', label: 'Create Account', icon: User },
  { key: 'server_check', label: 'Server Check', icon: Server },
  { key: 'base_domain', label: 'Base Domain', icon: Globe },
  { key: 'git_provider', label: 'Git Provider', icon: GitBranch },
  { key: 'first_app', label: 'First App', icon: Rocket },
];

export default function OnboardingWizard() {
  const [status, setStatus] = useState<OnboardingStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [baseDomain, setBaseDomain] = useState('');
  const [appName, setAppName] = useState('');
  const [gitRepo, setGitRepo] = useState('');
  const [checks, setChecks] = useState<any[]>([]);
  const [error, setError] = useState('');
  const [deploying, setDeploying] = useState(false);

  useEffect(() => {
    fetchStatus();
  }, []);

  async function fetchStatus() {
    try {
      const data = await api.getOnboardingStatus();
      if (data.is_complete) {
        window.location.href = '/';
        return;
      }
      if (data.current_step === 'first_deploy') {
        data.current_step = 'completed';
      }
      setStatus(data);
    } catch {}
    setLoading(false);
  }

  async function apiPost(path: string, body?: object) {
    setSubmitting(true);
    setError('');
    try {
      const data = await api.onboardingAction(path, body);
      setSubmitting(false);
      return data;
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Connection failed');
      setSubmitting(false);
      return null;
    }
  }

  async function handleAdminCreate() {
    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }
    const result = await apiPost('/onboarding/admin', { email, password });
    if (result) fetchStatus();
  }

  async function handleServerCheck() {
    const result = await apiPost('/onboarding/server-check');
    if (result) {
      setChecks(result.checks || []);
      if (result.all_passed) setTimeout(fetchStatus, 500);
    }
  }

  async function handleDomainSave() {
    if (!baseDomain.trim()) {
      await skipStep('base_domain');
      return;
    }
    const result = await apiPost('/onboarding/domain', { base_domain: baseDomain });
    if (result) fetchStatus();
  }

  async function handleGitSkip() {
    await skipStep('git_provider');
  }

  async function handleCreateApp() {
    setDeploying(true);
    const result = await apiPost('/onboarding/app', {
      name: appName,
      git_repo: gitRepo || undefined,
    });
    if (result) {
      await apiPost('/onboarding/first_deploy/complete');
      await apiPost('/onboarding/complete');
      window.location.href = '/';
    } else {
      setDeploying(false);
    }
  }

  async function handleSkipApp() {
    await apiPost('/onboarding/skip/first_app');
    await apiPost('/onboarding/skip/first_deploy');
    await apiPost('/onboarding/complete');
    window.location.href = '/';
  }

  async function skipStep(step: string) {
    await apiPost(`/onboarding/skip/${step}`);
    fetchStatus();
  }

  async function handleComplete() {
    await apiPost('/onboarding/complete');
    window.location.href = '/';
  }

  if (loading) return <div class={styles.loadingWrapper}>Loading...</div>;
  if (!status) return null;

  const currentStep = status.current_step;
  const stepIndex = STEPS.findIndex((s) => s.key === currentStep);
  const stepConfig = STEPS[stepIndex];

  if (currentStep === 'completed') {
    return (
      <div class={styles.cardCentered}>
        <div class={styles.successIcon}>
          <Check size={28} />
        </div>
        <h2 class={styles.completedTitle}>You're all set!</h2>
        <p class={styles.completedDescription}>Icefall is configured and ready to deploy.</p>
        <Button variant="primary" fullWidth onClick={handleComplete}>
          Go to Dashboard <ArrowRight size={14} />
        </Button>
      </div>
    );
  }

  return (
    <div>
      <div class={styles.card}>
        {stepConfig && (
          <div class={styles.stepHeader}>
            <div class={styles.stepIcon}>
              <stepConfig.icon size={24} />
            </div>
            <h2 class={styles.stepTitle}>
              {stepConfig.label}
            </h2>
          </div>
        )}

        {error && (
          <div class={styles.errorBanner} role="alert">
            {error}
          </div>
        )}

        {currentStep === 'admin_account' && (
          <div class={formStyles.fieldGroup}>
            <p class={styles.stepDescription}>Create your admin account to get started.</p>
            <div>
              <label htmlFor="onboard-email" class={formStyles.label}>Email</label>
              <input id="onboard-email" class={formStyles.input} type="email" autoComplete="email" aria-invalid={!!error && error.toLowerCase().includes('email')} value={email} onInput={(e) => setEmail((e.target as HTMLInputElement).value)} placeholder="admin@example.com" />
            </div>
            <div>
              <label htmlFor="onboard-password" class={formStyles.label}>Password</label>
              <input id="onboard-password" class={formStyles.input} type="password" autoComplete="new-password" aria-describedby="password-hint" aria-invalid={!!error && error.toLowerCase().includes('password')} value={password} onInput={(e) => setPassword((e.target as HTMLInputElement).value)} />
              <p id="password-hint" class={formStyles.hint}>Minimum 8 characters</p>
            </div>
            <div>
              <label htmlFor="onboard-confirm-password" class={formStyles.label}>Confirm Password</label>
              <input id="onboard-confirm-password" class={formStyles.input} type="password" autoComplete="new-password" aria-invalid={!!error && error.toLowerCase().includes('password')} value={confirmPassword} onInput={(e) => setConfirmPassword((e.target as HTMLInputElement).value)} />
            </div>
            <Button variant="primary" fullWidth onClick={handleAdminCreate} loading={submitting} disabled={!email || !password || !confirmPassword}>
              Create Account
            </Button>
          </div>
        )}

        {currentStep === 'server_check' && (
          <div class={formStyles.fieldGroup}>
            <p class={styles.stepDescription}>Checking your server configuration.</p>
            {checks.length > 0 ? (
              <div class={styles.checksList}>
                {checks.map((c: any) => (
                  <div key={c.id} class={styles.checkItem}>
                    {/* a11y [WCAG 1.4.1]: color-only dot supplemented with sr-only text */}
                    <span class={c.status === 'pass' ? styles.checkDotPass : c.status === 'warn' ? styles.checkDotWarn : styles.checkDotFail} aria-hidden="true" />
                    <span class="sr-only">{c.status}</span>
                    <span class={styles.checkName}>{c.name}</span>
                    <span class={styles.checkMessage}>{c.message}</span>
                  </div>
                ))}
              </div>
            ) : null}
            <Button variant="primary" fullWidth onClick={handleServerCheck} loading={submitting}>
              {checks.length > 0 ? 'Re-run Checks' : 'Run Checks'}
            </Button>
          </div>
        )}

        {currentStep === 'base_domain' && (
          <div class={formStyles.fieldGroup}>
            <p class={styles.stepDescription}>Configure a domain for HTTPS and app subdomains.</p>
            <Input
              label="Base Domain"
              name="onboard-base-domain"
              id="onboard-base-domain"
              mono
              value={baseDomain}
              onChange={setBaseDomain}
              placeholder="apps.example.com"
            />
            <Button variant="primary" fullWidth onClick={handleDomainSave} loading={submitting}>
              {baseDomain.trim() ? 'Continue' : 'Skip, add domain later'}
            </Button>
            {baseDomain.trim() && (
              <button onClick={() => skipStep('base_domain')} class={styles.skipButton}>
                Skip for now
              </button>
            )}
          </div>
        )}

        {currentStep === 'git_provider' && (
          <div class={formStyles.fieldGroup}>
            <p class={styles.stepDescription}>Link GitHub or GitLab for automatic deployments.</p>
            <div class={formStyles.fieldGroup}>
              <div class={styles.providerItem}>
                <svg class={styles.providerIcon} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                  <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
                </svg>
                <div class={styles.providerInfo}>
                  <p class={styles.providerName}>GitHub</p>
                  <p class={styles.providerHint}>Connect repositories from GitHub</p>
                </div>
                <Button variant="primary" size="sm">Connect</Button>
              </div>
              <div class={styles.providerItem}>
                <svg class={styles.providerIcon} viewBox="100 108 180 160" aria-hidden="true">
                  <path fill="#e24329" d="M265.26416,174.37243l-.2134-.55822-21.19899-55.30908c-.4236-1.08359-1.18542-1.99642-2.17699-2.62689-.98837-.63373-2.14749-.93253-3.32305-.87014-1.1689.06239-2.29195.48925-3.20809,1.21821-.90957.73554-1.56629,1.73047-1.87493,2.85346l-14.31327,43.80662h-57.90965l-14.31327-43.80662c-.30864-1.12299-.96536-2.11791-1.87493-2.85346-.91614-.72895-2.03911-1.15582-3.20809-1.21821-1.17548-.06239-2.33468.23641-3.32297.87014-.99166.63047-1.75348,1.5433-2.17707,2.62689l-21.19891,55.31237-.21348.55493c-6.28158,16.38521-.92929,34.90803,13.05891,45.48782.02621.01641.04922.03611.07552.05582l.18719.14119,32.29094,24.17392,15.97151,12.09024,9.71951,7.34871c2.34117,1.77316,5.57877,1.77316,7.92002,0l9.71943-7.34871,15.96822-12.09024,32.48142-24.31511c.02958-.02299.05588-.04269.08538-.06568,13.97834-10.57977,19.32735-29.09604,13.04905-45.47796Z" />
                  <path fill="#fc6d26" d="M265.26416,174.37243l-.2134-.55822c-10.5174,2.16062-20.20405,6.6099-28.49844,12.81593-.1346.0985-25.20497,19.05805-46.55171,35.19699,15.84998,11.98517,29.6477,22.40405,29.6477,22.40405l32.48142-24.31511c.02958-.02299.05588-.04269.08538-.06568,13.97834-10.57977,19.32735-29.09604,13.04905-45.47796Z" />
                  <path fill="#fca326" d="M160.34962,244.23117l15.97151,12.09024,9.71951,7.34871c2.34117,1.77316,5.57877,1.77316,7.92002,0l9.71943-7.34871,15.96822-12.09024s-13.79772-10.41888-29.6477-22.40405c-15.85327,11.98517-29.65099,22.40405-29.65099,22.40405Z" />
                  <path fill="#fc6d26" d="M143.44561,186.63014c-8.29111-6.20274-17.97446-10.65531-28.49507-12.81264l-.21348.55493c-6.28158,16.38521-.92929,34.90803,13.05891,45.48782.02621.01641.04922.03611.07552.05582l.18719.14119,32.29094,24.17392s13.79772-10.41888,29.65099-22.40405c-21.34673-16.13894-46.42031-35.09848-46.55499-35.19699Z" />
                </svg>
                <div class={styles.providerInfo}>
                  <p class={styles.providerName}>GitLab</p>
                  <p class={styles.providerHint}>Connect repositories from GitLab</p>
                </div>
                <Button variant="primary" size="sm">Connect</Button>
              </div>
            </div>
            <Button variant="primary" fullWidth onClick={handleGitSkip}>
              Continue
            </Button>
            <button onClick={handleGitSkip} class={styles.skipButton}>
              Skip for now
            </button>
          </div>
        )}

        {currentStep === 'first_app' && !deploying && (
          <div class={formStyles.fieldGroup}>
            <p class={styles.stepDescription}>Deploy your first app, or skip and do it later from the dashboard.</p>
            <Input
              label="Repository URL"
              name="onboard-git-repo"
              id="onboard-git-repo"
              mono
              value={gitRepo}
              onChange={setGitRepo}
              placeholder="https://github.com/user/repo"
            />
            <Input
              label="App Name"
              name="onboard-app-name"
              id="onboard-app-name"
              value={appName}
              onChange={setAppName}
              placeholder="my-awesome-app"
            />
            <Button variant="primary" fullWidth onClick={handleCreateApp} loading={submitting} disabled={!appName.trim()}>
              <Rocket size={14} /> Deploy
            </Button>
            <button onClick={handleSkipApp} class={styles.skipButton}>
              Skip, deploy later from the dashboard
            </button>
          </div>
        )}

        {currentStep === 'first_app' && deploying && (
          <div class={styles.loadingWrapper}>
            <p class={styles.deployMessage}>Your app is being deployed...</p>
            {/* a11y: prefers-reduced-motion handled in CSS module */}
            <div class={styles.deploySpinner} role="status" aria-label="Deploying" />
            <p class={styles.deployHint}>This may take a few minutes.</p>
          </div>
        )}
      </div>

      <div class={styles.stepIndicator} role="group" aria-label="Setup progress">
        <div class={styles.stepDots}>
          {STEPS.map((s, i) => (
            <span
              key={s.key}
              class={`${styles.dot} ${i < stepIndex ? styles.dotComplete : ''} ${i === stepIndex ? styles.dotActive : ''}`}
              aria-hidden="true"
            />
          ))}
        </div>
        <span class={styles.stepLabel}>Step {stepIndex + 1} of {STEPS.length}</span>
      </div>
    </div>
  );
}
