import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import styles from './inactive-apps-badge.module.css';

// IF-189: small badge on the Apps nav item showing how many apps have had no
// recent activity (no deploy in 90d / no inbound request in 30d).
export default function InactiveAppsBadge() {
  const [count, setCount] = useState(0);

  useEffect(() => {
    let active = true;
    api
      .listInactiveApps()
      .then(({ count }) => {
        if (active) setCount(count);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);

  if (count === 0) return null;

  return (
    <span
      class={styles.badge}
      title={`${count} inactive app${count === 1 ? '' : 's'}`}
      aria-label={`${count} inactive app${count === 1 ? '' : 's'}`}
    >
      {count}
    </span>
  );
}
