import { Search } from 'lucide-preact';
import { SERVICE_ICONS } from '../service-icons';
import styles from '../app-create.module.css';

type OneClickService = {
  name: string;
  description: string;
  image: string;
  port: number;
  env?: Record<string, string>;
  category: 'database' | 'cache' | 'analytics' | 'storage' | 'monitoring' | 'search';
};

export const ONE_CLICK_SERVICES: OneClickService[] = [
  { name: 'PostgreSQL', description: 'Relational database', image: 'postgres:16-alpine', port: 5432, env: { POSTGRES_PASSWORD: 'changeme', POSTGRES_DB: 'app' }, category: 'database' },
  { name: 'MySQL', description: 'Relational database', image: 'mysql:8', port: 3306, env: { MYSQL_ROOT_PASSWORD: 'changeme', MYSQL_DATABASE: 'app' }, category: 'database' },
  { name: 'MariaDB', description: 'MySQL-compatible database', image: 'mariadb:11', port: 3306, env: { MARIADB_ROOT_PASSWORD: 'changeme', MARIADB_DATABASE: 'app' }, category: 'database' },
  { name: 'MongoDB', description: 'Document database', image: 'mongo:7', port: 27017, env: { MONGO_INITDB_ROOT_USERNAME: 'root', MONGO_INITDB_ROOT_PASSWORD: 'changeme' }, category: 'database' },
  { name: 'Redis', description: 'In-memory data store', image: 'redis:7-alpine', port: 6379, category: 'cache' },
  { name: 'Valkey', description: 'Redis-compatible key-value store', image: 'valkey/valkey:8-alpine', port: 6379, category: 'cache' },
  { name: 'KeyDB', description: 'Multi-threaded Redis fork', image: 'eqalpha/keydb:latest', port: 6379, category: 'cache' },
  { name: 'Memcached', description: 'Distributed memory cache', image: 'memcached:1-alpine', port: 11211, category: 'cache' },
  { name: 'MinIO', description: 'S3-compatible object storage', image: 'minio/minio:latest', port: 9000, env: { MINIO_ROOT_USER: 'minio', MINIO_ROOT_PASSWORD: 'changeme' }, category: 'storage' },
  { name: 'Plausible', description: 'Privacy-friendly analytics', image: 'ghcr.io/plausible/community-edition:v2-latest', port: 8000, env: { BASE_URL: 'http://localhost:8000', SECRET_KEY_BASE: 'replace-me-with-64-chars' }, category: 'analytics' },
  { name: 'Umami', description: 'Simple web analytics', image: 'ghcr.io/umami-software/umami:postgresql-latest', port: 3000, env: { DATABASE_URL: 'postgresql://umami:changeme@localhost:5432/umami' }, category: 'analytics' },
  { name: 'Grafana', description: 'Observability dashboards', image: 'grafana/grafana-oss:latest', port: 3000, category: 'monitoring' },
  { name: 'Uptime Kuma', description: 'Uptime monitoring', image: 'louislam/uptime-kuma:1', port: 3001, category: 'monitoring' },
  { name: 'Meilisearch', description: 'Fast search engine', image: 'getmeili/meilisearch:latest', port: 7700, env: { MEILI_MASTER_KEY: 'changeme' }, category: 'search' },
  { name: 'Typesense', description: 'Search engine', image: 'typesense/typesense:27.1', port: 8108, env: { TYPESENSE_API_KEY: 'changeme' }, category: 'search' },
  { name: 'n8n', description: 'Workflow automation', image: 'n8nio/n8n:latest', port: 5678, category: 'monitoring' },
  { name: 'ClickHouse', description: 'Column-oriented analytics DB', image: 'clickhouse/clickhouse-server:latest', port: 8123, category: 'database' },
];

type Props = {
  serviceSearch: string;
  onSearchChange: (value: string) => void;
  filteredServices: OneClickService[];
  deployingService: string | null;
  onDeploy: (service: OneClickService) => void;
};

export default function OneClickServices({
  serviceSearch,
  onSearchChange,
  filteredServices,
  deployingService,
  onDeploy,
}: Props) {
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
          value={serviceSearch}
          onInput={(e) => onSearchChange((e.target as HTMLInputElement).value)}
          aria-label="Search one-click services"
        />
      </div>

      <div class={styles.serviceGrid}>
        {filteredServices.map((service) => (
          <button
            key={service.name}
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
          </button>
        ))}
        {filteredServices.length === 0 && (
          <p class={styles.noResults}>No services match "{serviceSearch}"</p>
        )}
      </div>
    </div>
  );
}
