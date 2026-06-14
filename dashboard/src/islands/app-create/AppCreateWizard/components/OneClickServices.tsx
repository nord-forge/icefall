import { useEffect, useState } from 'preact/hooks';
import { Search } from 'lucide-preact';
import { request } from '@lib/api';
import Badge from '@islands/shared/Badge/Badge';
import { SERVICE_ICONS } from '../service-icons';
import styles from '../app-create.module.css';

export type OneClickService = {
  id: string;
  name: string;
  description: string | null;
  version: string | null;
  categories: string | null;
  compose_content: string | null;
  default_env: string | null;
};

const CATEGORIES = ['All', 'AI/ML', 'Analytics', 'CMS', 'Communication', 'Database', 'DevTools', 'Media', 'Monitoring', 'Productivity', 'Security', 'Storage'];

type Props = {
  deployingService: string | null;
  onDeploy: (service: OneClickService) => void;
};

export default function OneClickServices({
  deployingService,
  onDeploy,
}: Props) {
  const [templates, setTemplates] = useState<OneClickService[]>([]);
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('All');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    request<{ data: OneClickService[] }>('/templates')
      .then(({ data }) => setTemplates(data))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const filtered = templates
    .filter((t) => {
      const matchesSearch = !search || t.name.toLowerCase().includes(search.toLowerCase())
        || (t.description || '').toLowerCase().includes(search.toLowerCase());
      const matchesCategory = category === 'All'
        || (t.categories || '').toLowerCase().includes(category.toLowerCase());
      return matchesSearch && matchesCategory;
    })
    .sort((a, b) => a.name.localeCompare(b.name));

  return (
    <div class={styles.oneClickSection}>
      <div class={styles.divider}>
        <span class={styles.dividerLabel}>or deploy a service</span>
      </div>

      <div class={styles.searchWrapper}>
        <Search size={16} class={styles.searchIcon} aria-hidden="true" />
        <input
          type="search"
          class={styles.searchInput}
          placeholder="Search services..."
          value={search}
          onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
          aria-label="Search one-click services"
        />
      </div>

      <div class={styles.categoryFilters} role="group" aria-label="Category filter">
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            type="button"
            class={`${styles.categoryChip} ${cat === category ? styles.categoryChipActive : ''}`}
            onClick={() => setCategory(cat)}
            aria-pressed={cat === category}
          >
            {cat}
          </button>
        ))}
      </div>

      {loading ? (
        <p class={styles.noResults}>Loading services...</p>
      ) : (
        <div class={styles.serviceGrid}>
          {filtered.map((service) => (
            <button
              key={service.id}
              type="button"
              class={styles.serviceCard}
              onClick={() => onDeploy(service)}
              disabled={deployingService !== null}
              aria-label={`Deploy ${service.name}`}
            >
              <span class={styles.serviceName}>
                {deployingService === service.name ? (
                  <span class={styles.deployingDot} />
                ) : SERVICE_ICONS[service.name] ? (
                  <span class={styles.serviceIcon}>
                    {SERVICE_ICONS[service.name]({ size: 16 })}
                  </span>
                ) : null}
                {service.name}
              </span>
              <span class={styles.serviceDescription}>{service.description}</span>
              {service.categories && (
                <span class={styles.serviceTags}>
                  {service.categories.split(',').map((c) => (
                    <Badge key={c} label={c.trim()} size="sm" />
                  ))}
                </span>
              )}
            </button>
          ))}
          {filtered.length === 0 && (
            <p class={styles.noResults}>
              {templates.length === 0 ? 'No templates available.' : `No services match "${search}"`}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
