import { useEffect, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $domains, $domainsLoaded } from '@stores/domains';
import type { DomainWithApp } from '@stores/domains';
import { api } from '@lib/api';
import type { App, Domain } from '@lib/types';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import { Globe, Shield, ExternalLink } from 'lucide-preact';
import styles from './domains-page.module.css';

export default function DomainsPage() {
  const cachedDomains = useStore($domains);
  const wasLoaded = useStore($domainsLoaded);
  const [domains, setDomains] = useState<DomainWithApp[]>(cachedDomains);
  const [loading, setLoading] = useState(!wasLoaded);

  useEffect(() => {
    async function load() {
      try {
        const { data: apps } = await api.listApps();
        const all: DomainWithApp[] = [];
        for (const app of apps) {
          try {
            const { data: appDomains } = await api.listDomains(app.id);
            all.push(...appDomains.map(d => ({ ...d, appName: app.name })));
          } catch {}
        }
        setDomains(all);
        $domains.set(all);
      } catch {}
      $domainsLoaded.set(true);
      setLoading(false);
    }
    load();
  }, []);

  return (
    <div>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Domains</h1>
      </div>

      {loading ? (
        <p class={styles.loadingText}>Loading domains...</p>
      ) : domains.length === 0 ? (
        <div class={styles.emptyState}>
          <Globe size={40} class={styles.emptyIcon} aria-hidden="true" />
          <p class={styles.emptyTitle}>No domains configured</p>
          <p class={styles.emptyHint}>Add domains from each app's Domains tab. <a href="/" data-astro-prefetch="hover">Browse your apps</a> to get started.</p>
        </div>
      ) : (
        <div class={styles.tableCard}>
          <table class={styles.table}>
            <thead>
              <tr class={styles.tableRow}>
                <th class={styles.th}>Domain</th>
                <th class={styles.th}>App</th>
                <th class={styles.th}>SSL</th>
                <th class={styles.th}>DNS</th>
              </tr>
            </thead>
            <tbody>
              {domains.map(d => (
                <tr key={d.id} class={styles.tableRow}>
                  <td class={styles.domainCell}>
                    <Shield size={14} aria-hidden="true" class={d.ssl_status === 'active' ? styles.sslActive : styles.sslInactive} />
                    {d.domain}
                  </td>
                  <td class={styles.td}>
                    <a href={`/apps/${d.app_id}/domains`} class={styles.appLink}>
                      {d.appName}
                      <ExternalLink size={12} aria-hidden="true" />
                    </a>
                  </td>
                  <td class={`${styles.td} ${d.ssl_status === 'active' ? styles.sslActive : styles.sslInactive}`}>
                    {d.ssl_status === 'active' ? 'Valid' : d.ssl_status}
                  </td>
                  <td class={styles.td}>
                    <span class={d.verified ? styles.dnsConfigured : styles.dnsPending}>
                      <span aria-hidden="true" class={styles.dnsDot} />
                      {d.verified ? 'Configured' : 'Pending'}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
