import { FlaskConical } from 'lucide-preact';
import styles from './canary-results-section.module.css';

type Props = {
  deployId: string;
  canaryEnabled: boolean;
};

type Verdict = 'passed' | 'failed' | 'running';

const VERDICT_LABELS: Record<Verdict, string> = {
  passed: 'Passed',
  failed: 'Failed',
  running: 'Running',
};

const VERDICT_CLASSES: Record<Verdict, string> = {
  passed: styles.verdictPassed,
  failed: styles.verdictFailed,
  running: styles.verdictRunning,
};

export default function CanaryResultsSection({
  deployId,
  canaryEnabled,
}: Props) {
  if (!canaryEnabled) return null;

  // Placeholder/mock data since the backend returns stubs
  const verdict: Verdict = 'passed';
  const metrics = {
    p50: 42,
    p95: 128,
    p99: 256,
    errorRate: 0.2,
    requestCount: 50,
  };

  return (
    <div class={styles.container}>
      <h3 class={styles.title}>
        <FlaskConical size={16} aria-hidden="true" />
        Canary Results
        <span
          class={`${styles.verdict} ${VERDICT_CLASSES[verdict]}`}
          role="status"
          aria-live="polite"
        >
          {VERDICT_LABELS[verdict]}
        </span>
      </h3>
      <div class={styles.metricsGrid}>
        <div class={styles.metric}>
          <span class={styles.metricLabel}>p50</span>
          <span class={styles.metricValue}>{metrics.p50}ms</span>
        </div>
        <div class={styles.metric}>
          <span class={styles.metricLabel}>p95</span>
          <span class={styles.metricValue}>{metrics.p95}ms</span>
        </div>
        <div class={styles.metric}>
          <span class={styles.metricLabel}>p99</span>
          <span class={styles.metricValue}>{metrics.p99}ms</span>
        </div>
        <div class={styles.metric}>
          <span class={styles.metricLabel}>Error rate</span>
          <span class={styles.metricValue}>{metrics.errorRate}%</span>
        </div>
        <div class={styles.metric}>
          <span class={styles.metricLabel}>Requests</span>
          <span class={styles.metricValue}>{metrics.requestCount}</span>
        </div>
      </div>
    </div>
  );
}
