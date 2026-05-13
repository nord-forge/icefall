import type { User } from '@lib/types';
import { User as UserIcon } from 'lucide-preact';
import styles from '../profile-page.module.css';

type Props = {
  user: User | null;
};

export default function AccountInfoSection({ user }: Props) {
  return (
    <section class={styles.section} aria-labelledby="account-info-heading">
      <h2 id="account-info-heading" class={styles.sectionHeading}>
        {/* a11y [1.1.1]: decorative icon hidden from assistive tech */}
        <UserIcon size={18} aria-hidden="true" /> Account Info
      </h2>
      <div class={styles.infoGrid}>
        <div class={styles.infoItem}>
          <span class={styles.infoLabel}>Email</span>
          <span class={styles.infoValue}>{user?.email || '-'}</span>
        </div>
        <div class={styles.infoItem}>
          <span class={styles.infoLabel}>Role</span>
          <span class={styles.roleBadge}>{user?.role || '-'}</span>
        </div>
        <div class={styles.infoItem}>
          <span class={styles.infoLabel}>Member since</span>
          <span class={styles.infoValue}>
            {user?.created_at ? new Date(user.created_at).toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' }) : '-'}
          </span>
        </div>
        <div class={styles.infoItem}>
          <span class={styles.infoLabel}>Two-Factor</span>
          <span class={styles.infoValue}>{user?.totp_enabled ? 'Enabled' : 'Disabled'}</span>
        </div>
      </div>
    </section>
  );
}
