import { useEffect, useState } from 'preact/hooks';
import { api, request } from '@lib/api';
import { addToast } from '@stores/toast';
import Card from '@islands/shared/Card/Card';
import Badge from '@islands/shared/Badge/Badge';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import styles from './incidents-page.module.css';

type Incident = {
  id: string;
  title: string;
  status: string;
  severity: string;
  affected_apps: string | null;
  started_at: string;
  resolved_at: string | null;
  created_at: string;
};

const severityVariant = (s: string) => {
  switch (s) {
    case 'critical': return 'error' as const;
    case 'major': return 'warning' as const;
    default: return 'info' as const;
  }
};

const statusVariant = (s: string) => {
  switch (s) {
    case 'resolved': return 'success' as const;
    case 'monitoring': return 'info' as const;
    case 'identified': return 'warning' as const;
    default: return 'default' as const;
  }
};

export default function IncidentsPage() {
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreate, setShowCreate] = useState(false);
  const [title, setTitle] = useState('');
  const [severity, setSeverity] = useState('minor');

  useEffect(() => {
    request<{ data: Incident[] }>('/incidents')
      .then(({ data }) => setIncidents(data))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  async function handleCreate() {
    if (!title.trim()) return;
    try {
      const { data } = await request<{ data: Incident }>('/incidents', {
        method: 'POST',
        body: JSON.stringify({ title, severity }),
      });
      setIncidents([data, ...incidents]);
      setTitle('');
      setShowCreate(false);
      addToast('success', 'Incident created');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to create incident');
    }
  }

  if (loading) return <p class={styles.loading}>Loading incidents...</p>;

  return (
    <div class={styles.page}>
      <div class={styles.header}>
        <h1 class={styles.title}>Incidents</h1>
        <Button variant="primary" onClick={() => setShowCreate(!showCreate)}>
          Create incident
        </Button>
      </div>

      {showCreate && (
        <Card title="New incident">
          <div class={styles.createForm}>
            <Input label="Title" name="title" value={title} onChange={setTitle} required />
            <div class={styles.severityGroup}>
              <label class={styles.label}>Severity</label>
              <div class={styles.severityOptions} role="radiogroup" aria-label="Severity">
                {['minor', 'major', 'critical'].map((s) => (
                  <button
                    key={s} type="button"
                    class={`${styles.severityBtn} ${s === severity ? styles.selected : ''}`}
                    onClick={() => setSeverity(s)}
                    role="radio" aria-checked={s === severity}
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>
            <Button variant="primary" onClick={handleCreate}>Create</Button>
          </div>
        </Card>
      )}

      {incidents.length === 0 ? (
        <p class={styles.empty}>No incidents recorded.</p>
      ) : (
        <div class={styles.list}>
          {incidents.map((inc) => (
            <Card key={inc.id}>
              <div class={styles.incidentRow}>
                <div>
                  <h3 class={styles.incidentTitle}>{inc.title}</h3>
                  <div class={styles.badges}>
                    <Badge label={inc.status} variant={statusVariant(inc.status)} />
                    <Badge label={inc.severity} variant={severityVariant(inc.severity)} />
                  </div>
                </div>
                <time class={styles.time}>{new Date(inc.created_at).toLocaleDateString()}</time>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
