import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import Card from '@islands/shared/Card/Card';
import CodeBlock from '@islands/shared/CodeBlock/CodeBlock';
import Toggle from '@islands/shared/Toggle/Toggle';
import Button from '@islands/shared/Button/Button';
import styles from './proxy-tab.module.css';

export default function ProxyTab({ appId }: { appId: string }) {
  const [config, setConfig] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.request<{ data: any }>(`/apps/${appId}/proxy`)
      .then(({ data }) => setConfig(JSON.stringify(data, null, 2)))
      .catch(() => setConfig('// No proxy config available'))
      .finally(() => setLoading(false));
  }, [appId]);

  if (loading) return <p class={styles.loading}>Loading proxy config...</p>;

  return (
    <div class={styles.page}>
      <Card title="Caddy route configuration">
        <CodeBlock code={config} language="json" />
      </Card>

      <Card title="Middleware presets">
        <div class={styles.presets}>
          <Toggle label="Force HTTPS" checked={true} onChange={() => {}} description="Redirect all HTTP to HTTPS (enabled by default via Caddy)" />
          <Toggle label="Rate limiting" checked={false} onChange={() => {}} description="Limit requests per IP address" />
          <Toggle label="HTTP Basic Auth" checked={false} onChange={() => {}} description="Password-protect the entire app" />
        </div>
      </Card>
    </div>
  );
}
