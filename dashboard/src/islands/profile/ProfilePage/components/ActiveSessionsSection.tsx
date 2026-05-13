import { formatRelativeTime } from '@lib/format';
import { Monitor, LogOut } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from '../profile-page.module.css';

type SessionEntry = {
  id: string;
  created_at: string;
  expires_at: string;
  is_current: boolean;
};

type Props = {
  sessions: SessionEntry[];
  sessionsLoading: boolean;
  sessionsMsg: string;
  onRevokeAllSessions: () => void;
};

export default function ActiveSessionsSection({ sessions, sessionsLoading, sessionsMsg, onRevokeAllSessions }: Props) {
  return (
    <section class={styles.section} aria-labelledby="sessions-heading">
      <div class={styles.sectionHeader}>
        <h2 id="sessions-heading" class={styles.sectionHeading}>
          <Monitor size={18} aria-hidden="true" /> Active Sessions
        </h2>
        {sessions.length > 1 && (
          <Button variant="ghost" size="sm" onClick={onRevokeAllSessions} loading={sessionsLoading}>
            <LogOut size={14} aria-hidden="true" /> Sign out everywhere
          </Button>
        )}
      </div>

      {sessionsMsg && <p class={styles.feedbackSuccess} role="status">{sessionsMsg}</p>}

      <div class={styles.tableCard}>
        <table class={styles.table}>
          <thead>
            <tr class={styles.tableRow}>
              <th class={styles.th}>Session</th>
              <th class={styles.th}>Created</th>
              <th class={styles.th}>Expires</th>
              <th class={styles.th}>Status</th>
            </tr>
          </thead>
          <tbody>
            {sessions.map(s => (
              <tr key={s.id} class={styles.tableRow}>
                <td class={styles.tdMono}>{s.id.slice(0, 8)}...</td>
                <td class={styles.tdMuted}>{formatRelativeTime(s.created_at)}</td>
                <td class={styles.tdMuted}>{new Date(s.expires_at).toLocaleDateString()}</td>
                <td class={styles.td}>
                  {s.is_current && (
                    <span class={styles.currentBadge}>Current</span>
                  )}
                </td>
              </tr>
            ))}
            {sessions.length === 0 && (
              <tr>
                <td class={styles.emptyRow} colSpan={4}>No active sessions.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
