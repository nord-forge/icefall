import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Card from '@islands/shared/Card/Card';
import Input from '@islands/shared/Input/Input';
import Toggle from '@islands/shared/Toggle/Toggle';
import Button from '@islands/shared/Button/Button';
import Badge from '@islands/shared/Badge/Badge';
import styles from './tasks-tab.module.css';

type Task = { id: string; name: string; command: string; cron_expression: string; timeout_seconds: number; enabled: boolean; };
type Execution = { id: string; status: string; output: string | null; started_at: string; finished_at: string | null; };

export default function TasksTab({ appId }: { appId: string }) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [executions, setExecutions] = useState<Record<string, Execution[]>>({});
  const [loading, setLoading] = useState(true);
  const [showAdd, setShowAdd] = useState(false);
  const [name, setName] = useState('');
  const [command, setCommand] = useState('');
  const [cron, setCron] = useState('');

  useEffect(() => {
    api.request<{ data: Task[] }>(`/apps/${appId}/scheduled-tasks`)
      .then(({ data }) => setTasks(data))
      .finally(() => setLoading(false));
  }, [appId]);

  async function handleCreate() {
    try {
      const { data } = await api.request<{ data: Task }>(`/apps/${appId}/scheduled-tasks`, {
        method: 'POST', body: JSON.stringify({ name, command, cron_expression: cron }),
      });
      setTasks([...tasks, data]);
      setShowAdd(false); setName(''); setCommand(''); setCron('');
      addToast('success', 'Task created');
    } catch (err: any) { addToast('error', err.message); }
  }

  async function handleRun(taskId: string) {
    try {
      await api.request(`/apps/${appId}/scheduled-tasks/${taskId}/run`, { method: 'POST' });
      addToast('info', 'Task started');
    } catch (err: any) { addToast('error', err.message); }
  }

  async function handleToggle(taskId: string, enabled: boolean) {
    await api.request(`/apps/${appId}/scheduled-tasks/${taskId}/toggle`, {
      method: 'POST', body: JSON.stringify({ enabled }),
    });
    setTasks(tasks.map(t => t.id === taskId ? { ...t, enabled } : t));
  }

  if (loading) return <p class={styles.loading}>Loading tasks...</p>;

  return (
    <div class={styles.page}>
      <div class={styles.header}>
        <h2 class={styles.title}>Scheduled tasks</h2>
        <Button variant="secondary" size="sm" onClick={() => setShowAdd(!showAdd)}>Add task</Button>
      </div>

      {showAdd && (
        <Card title="New task">
          <div class={styles.form}>
            <Input label="Name" name="task-name" value={name} onChange={setName} placeholder="Database vacuum" />
            <Input label="Command" name="task-cmd" value={command} onChange={setCommand} placeholder="psql -c 'VACUUM'" />
            <Input label="Cron expression" name="task-cron" value={cron} onChange={setCron} placeholder="0 3 * * *" helpText="minute hour day month weekday" />
            <Button variant="primary" onClick={handleCreate}>Create task</Button>
          </div>
        </Card>
      )}

      {tasks.length === 0 ? (
        <p class={styles.empty}>No scheduled tasks. Add one to automate maintenance.</p>
      ) : (
        tasks.map(task => (
          <Card key={task.id}>
            <div class={styles.taskRow}>
              <div>
                <strong class={styles.taskName}>{task.name}</strong>
                <code class={styles.taskCron}>{task.cron_expression}</code>
                <code class={styles.taskCmd}>{task.command}</code>
              </div>
              <div class={styles.taskActions}>
                <Toggle label="" checked={task.enabled} onChange={(v) => handleToggle(task.id, v)} />
                <Button variant="ghost" size="sm" onClick={() => handleRun(task.id)}>Run now</Button>
              </div>
            </div>
          </Card>
        ))
      )}
    </div>
  );
}
