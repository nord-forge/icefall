import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import { Grid3x3, Database, Folder, Server } from 'lucide-preact';
import type { LucideIcon } from 'lucide-preact';
import styles from './counts-strip.module.css';

type Counts = {
  apps: number | null;
  databases: number | null;
  projects: number | null;
  servers: number | null;
};

type Item = {
  key: keyof Counts;
  label: string;
  href: string;
  icon: LucideIcon;
};

const ITEMS: Item[] = [
  { key: 'apps', label: 'Apps', href: '/', icon: Grid3x3 },
  { key: 'databases', label: 'Databases', href: '/databases', icon: Database },
  { key: 'projects', label: 'Projects', href: '/projects', icon: Folder },
  { key: 'servers', label: 'Servers', href: '/servers', icon: Server },
];

export default function CountsStrip() {
  const [counts, setCounts] = useState<Counts>({ apps: null, databases: null, projects: null, servers: null });
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let active = true;
    // Each count fails independently — a dead endpoint shows "—", not a blank strip
    async function load() {
      const [apps, databases, projects, servers] = await Promise.all([
        api.listApps().then((r) => r.data.length).catch(() => null),
        api.listDatabases().then((r) => r.data.length).catch(() => null),
        api.listProjects().then((r) => r.data.length).catch(() => null),
        api.listServers().then((r) => r.data.length).catch(() => null),
      ]);
      if (active) {
        setCounts({ apps, databases, projects, servers });
        setLoaded(true);
      }
    }
    load();
    return () => { active = false; };
  }, []);

  return (
    <dl class={styles.strip}>
      {ITEMS.map(({ key, label, href, icon: Icon }) => (
        <a key={key} href={href} class={styles.item}>
          <Icon size={18} aria-hidden="true" class={styles.icon} />
          <div class={styles.text}>
            <dd class={styles.value}>
              {!loaded ? <span class={styles.skeleton} aria-hidden="true" /> : counts[key] ?? '—'}
            </dd>
            <dt class={styles.label}>{label}</dt>
          </div>
        </a>
      ))}
    </dl>
  );
}
