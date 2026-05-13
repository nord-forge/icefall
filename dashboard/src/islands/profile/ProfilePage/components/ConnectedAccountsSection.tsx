import { Link2 } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from '../profile-page.module.css';

type OAuthIdentity = {
  id: string;
  provider: string;
  provider_email: string | null;
  created_at: string;
};

type Props = {
  identities: OAuthIdentity[];
  enabledProviders: { github: boolean; google: boolean };
  onUnlink: (provider: string) => Promise<void>;
};

export default function ConnectedAccountsSection({ identities, enabledProviders, onUnlink }: Props) {
  return (
    <section class={styles.section} aria-labelledby="oauth-heading">
      <h2 id="oauth-heading" class={styles.sectionHeading}>
        <Link2 size={18} aria-hidden="true" /> Connected Accounts
      </h2>
      <p class={styles.sectionDescription}>
        Link third-party accounts for faster sign-in. Providers can be configured in <a href="/settings#oauth" class={styles.inlineLink}>Settings</a>.
      </p>

      <div class={styles.providerList}>
        {(['github', 'google'] as const).map(provider => {
          const identity = identities.find(i => i.provider === provider);
          const enabled = enabledProviders[provider];

          return (
            <div key={provider} class={styles.providerRow}>
              <div class={styles.providerInfo}>
                <span class={styles.providerName}>{provider}</span>
                {identity ? (
                  <span class={styles.providerEmail}>{identity.provider_email || 'Linked'}</span>
                ) : (
                  <span class={styles.providerNotLinked}>Not linked</span>
                )}
              </div>
              {identity ? (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={async () => {
                    try {
                      await onUnlink(provider);
                    } catch {}
                  }}
                >
                  Unlink
                </Button>
              ) : enabled ? (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    window.location.href = `/api/v1/auth/oauth/${provider}/authorize`;
                  }}
                >
                  Link {provider}
                </Button>
              ) : (
                <span class={styles.providerNotLinked}>Not configured</span>
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}
