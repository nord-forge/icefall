import { useEffect, useState } from 'preact/hooks';
import { api, request } from '@lib/api';
import { Search } from 'lucide-preact';
import Badge from '@islands/shared/Badge/Badge';
import Button from '@islands/shared/Button/Button';
import { SERVICE_ICONS } from '@islands/app-create/AppCreateWizard/service-icons';
import styles from './services-page.module.css';

type Template = {
  id: string;
  name: string;
  description: string | null;
  version: string | null;
  icon_url: string | null;
  categories: string | null;
  website: string | null;
};

const CATEGORIES = ['All', 'AI/ML', 'Analytics', 'CMS', 'Communication', 'Database', 'DevTools', 'Media', 'Monitoring', 'Productivity', 'Security', 'Storage'];

export default function ServicesPage() {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('All');

  useEffect(() => {
    request<{ data: Template[] }>('/templates')
      .then(({ data }) => setTemplates(data))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const filtered = templates.filter((t) => {
    const matchesSearch = !search || t.name.toLowerCase().includes(search.toLowerCase());
    const matchesCategory = category === 'All' || (t.categories || '').toLowerCase().includes(category.toLowerCase());
    return matchesSearch && matchesCategory;
  }).sort((a, b) => a.name.localeCompare(b.name));

  if (loading) return <p class={styles.loading}>Loading templates...</p>;

  return (
    <div class={styles.page}>
      <div class={styles.header}>
        <h1 class={styles.title}>Services</h1>
        <div class={styles.searchWrapper}>
          <Search size={16} class={styles.searchIcon} aria-hidden="true" />
          <input
            type="search"
            class={styles.searchInput}
            placeholder="Search services..."
            value={search}
            onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
            aria-label="Search services"
          />
        </div>
      </div>

      <div class={styles.categories} role="group" aria-label="Category filter">
        {CATEGORIES.map((cat) => (
          <button
            key={cat} type="button"
            class={`${styles.categoryBtn} ${cat === category ? styles.active : ''}`}
            onClick={() => setCategory(cat)}
            aria-pressed={cat === category}
          >
            {cat}
          </button>
        ))}
      </div>

      {filtered.length === 0 ? (
        <p class={styles.empty}>
          {templates.length === 0 ? 'No templates available. Check back after refreshing the catalog.' : 'No services match your search.'}
        </p>
      ) : (
        <div class={styles.grid}>
          {filtered.map((t) => (
            <div key={t.id} class={styles.card}>
              <div class={styles.cardHeader}>
                {SERVICE_ICONS[t.name]
                  ? SERVICE_ICONS[t.name]({ size: 32 })
                  : <div class={styles.iconPlaceholder} />
                }
                <div class={styles.cardTitleGroup}>
                  <h3 class={styles.cardTitle}>{t.name}</h3>
                  {t.version && <span class={styles.version}>v{t.version}</span>}
                </div>
                {t.categories && (
                  <div class={styles.badges}>
                    {t.categories.split(',').map((c) => (
                      <Badge key={c} label={c.trim()} size="sm" />
                    ))}
                  </div>
                )}
              </div>
              {t.description && <p class={styles.description}>{t.description}</p>}
              <div class={styles.cardActions}>
                <Button variant="primary" size="sm">Deploy</Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
