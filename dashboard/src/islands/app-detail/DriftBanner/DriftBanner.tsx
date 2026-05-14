import { useState, useEffect } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { AlertTriangle, Rocket } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from './drift-banner.module.css';

type Props = {
  appId: string;
  onRedeploy?: () => void;
};

export default function DriftBanner({ appId, onRedeploy }: Props) {
  const [drifted, setDrifted] = useState(false);
  const [deploying, setDeploying] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function check() {
      try {
        const { data } = await api.checkDrift(appId);
        if (!cancelled) setDrifted(data.drifted);
      } catch {
        // silently ignore — drift check is non-critical
      }
    }

    check();
    return () => { cancelled = true; };
  }, [appId]);

  if (!drifted) return null;

  async function handleRedeploy() {
    setDeploying(true);
    try {
      await api.triggerDeploy(appId);
      setDrifted(false);
      onRedeploy?.();
      window.location.href = `/apps/${appId}/deploys`;
    } catch (err: any) {
      addToast('error', err.message || 'Failed to trigger deploy');
      setDeploying(false);
    }
  }

  return (
    <div class={styles.banner} role="alert">
      <div class={styles.content}>
        <AlertTriangle size={16} aria-hidden="true" />
        <span>Configuration has changed since the last deploy. Redeploy to apply changes.</span>
      </div>
      <Button variant="secondary" size="sm" onClick={handleRedeploy} loading={deploying}>
        <Rocket size={14} /> Redeploy Now
      </Button>
    </div>
  );
}
