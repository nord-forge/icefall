import { useState } from 'preact/hooks';
import type { App, LbPolicy } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { Layers } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import Toggle from '@islands/shared/Toggle/Toggle';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  app: App;
};

const MIN_INSTANCES = 1;
const MAX_INSTANCES = 10;

const LB_POLICY_OPTIONS: { value: LbPolicy; label: string }[] = [
  { value: 'round_robin', label: 'Round robin' },
  { value: 'least_conn', label: 'Least connections' },
  { value: 'ip_hash', label: 'IP hash (client affinity)' },
  { value: 'random', label: 'Random' },
];

export default function ScalingCard({ app }: Props) {
  const [desired, setDesired] = useState(String(app.desired_instances ?? 1));
  const [policy, setPolicy] = useState<LbPolicy>(app.lb_policy ?? 'round_robin');
  const [healthPath, setHealthPath] = useState(app.lb_health_check_path ?? '/');
  const [sticky, setSticky] = useState(!!app.lb_sticky_sessions);
  const [savingLb, setSavingLb] = useState(false);
  const [scaling, setScaling] = useState(false);

  const desiredNum = parseInt(desired, 10);
  const desiredValid =
    !Number.isNaN(desiredNum) &&
    desiredNum >= MIN_INSTANCES &&
    desiredNum <= MAX_INSTANCES;
  const healthValid = healthPath.startsWith('/');

  async function handleScale() {
    if (!desiredValid) return;
    setScaling(true);
    try {
      await api.scaleApp(app.id, desiredNum);
      addToast(
        'success',
        desiredNum === app.desired_instances
          ? 'Redeploying instances'
          : `Scaling to ${desiredNum} instance${desiredNum === 1 ? '' : 's'}`,
      );
    } catch (err: any) {
      addToast('error', err.message || 'Failed to scale app');
    }
    setScaling(false);
  }

  async function handleSaveLb() {
    if (!healthValid) return;
    setSavingLb(true);
    try {
      await api.updateLbConfig(app.id, {
        policy,
        health_check_path: healthPath,
        sticky_sessions: sticky,
      });
      addToast('success', 'Load balancing settings updated');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to update load balancing');
    }
    setSavingLb(false);
  }

  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <Layers size={18} aria-hidden="true" /> Scaling &amp; Load Balancing
      </h2>
      <p class={styles.settingsDescription}>
        Run multiple instances of this app across your servers. Traffic is
        distributed by the reverse proxy using the policy below.
      </p>

      <div class={formStyles.fieldRow}>
        <Input
          label="Desired instances"
          name="desired-instances"
          id="scaling-desired-instances"
          type="number"
          min={MIN_INSTANCES}
          max={MAX_INSTANCES}
          value={desired}
          onChange={setDesired}
          error={
            desired && !desiredValid
              ? `Must be between ${MIN_INSTANCES} and ${MAX_INSTANCES}`
              : undefined
          }
          helpText={`How many copies to run (${MIN_INSTANCES}–${MAX_INSTANCES}).`}
        />
      </div>
      <div class={styles.saveRow}>
        <Button
          variant="primary"
          onClick={handleScale}
          loading={scaling}
          disabled={!desiredValid}
        >
          Apply scaling
        </Button>
      </div>

      <div class={formStyles.fieldRow}>
        <div>
          {/* a11y [WCAG 1.3.1]: explicit label associated with the select */}
          <label class={formStyles.label} for="scaling-lb-policy">
            Load balancing policy
          </label>
          <Select
            id="scaling-lb-policy"
            options={LB_POLICY_OPTIONS}
            value={policy}
            onChange={(v) => setPolicy(v as LbPolicy)}
            fullWidth
          />
        </div>
        <Input
          label="Health check path"
          name="lb-health-path"
          id="scaling-health-path"
          type="text"
          value={healthPath}
          onChange={setHealthPath}
          error={healthPath && !healthValid ? "Must start with '/'" : undefined}
          helpText="Path the proxy probes to detect unhealthy instances."
        />
      </div>

      <Toggle
        label="Sticky sessions"
        description="Pin each client to one instance for the duration of their session."
        checked={sticky}
        onChange={setSticky}
      />

      <div class={styles.saveRow}>
        <Button
          variant="secondary"
          onClick={handleSaveLb}
          loading={savingLb}
          disabled={!healthValid}
        >
          Save load balancing settings
        </Button>
      </div>
      <p class={styles.settingsNote}>
        Scaling triggers a new deployment. Load balancing changes apply to the
        running proxy immediately.
      </p>
    </div>
  );
}
