import type { AppStatus, DeployStatus } from '@lib/types';
import styles from './status-dot.module.css';

const STATUS_LABELS: Record<string, string> = {
  online: 'Online',
  running: 'Running',
  building: 'Building',
  deploying: 'Deploying',
  pending: 'Pending',
  failed: 'Failed',
  stopped: 'Stopped',
  cancelled: 'Cancelled',
  success: 'Success',
  offline: 'Offline',
  unreachable: 'Unreachable',
  enrolling: 'Enrolling',
  draining: 'Draining',
};

type StatusShape = 'filled' | 'ring' | 'pulse' | 'x' | 'triangle' | 'half' | 'dash';

const STATUS_SHAPES: Record<string, StatusShape> = {
  online: 'filled',
  running: 'filled',
  success: 'filled',
  building: 'pulse',
  deploying: 'pulse',
  pending: 'dash',
  failed: 'x',
  stopped: 'ring',
  cancelled: 'dash',
  offline: 'ring',
  unreachable: 'ring',
  enrolling: 'triangle',
  draining: 'half',
};

type Props = {
  status: AppStatus | DeployStatus | string;
  showLabel?: boolean;
}

function ShapeIcon({ shape, colorClass }: { shape: StatusShape; colorClass: string }) {
  const size = 10;

  switch (shape) {
    case 'filled':
      return (
        <svg class={`${styles.icon} ${colorClass}`} width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
          <circle cx="5" cy="5" r="4" fill="currentColor" />
        </svg>
      );
    case 'ring':
      return (
        <svg class={`${styles.icon} ${colorClass}`} width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
          <circle cx="5" cy="5" r="3.5" fill="none" stroke="currentColor" stroke-width="1.5" />
        </svg>
      );
    case 'pulse':
      return (
        <svg class={`${styles.icon} ${styles.pulseAnim} ${colorClass}`} width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
          <circle cx="5" cy="5" r="2.5" fill="currentColor" />
          <circle cx="5" cy="5" r="4" fill="none" stroke="currentColor" stroke-width="1" opacity="0.4" />
        </svg>
      );
    case 'x':
      return (
        <svg class={`${styles.icon} ${colorClass}`} width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
          <line x1="2.5" y1="2.5" x2="7.5" y2="7.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          <line x1="7.5" y1="2.5" x2="2.5" y2="7.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      );
    case 'triangle':
      return (
        <svg class={`${styles.icon} ${colorClass}`} width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
          <polygon points="5,1.5 9,8.5 1,8.5" fill="currentColor" />
        </svg>
      );
    case 'half':
      return (
        <svg class={`${styles.icon} ${colorClass}`} width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
          <path d="M5 1 A4 4 0 0 1 5 9" fill="currentColor" />
          <path d="M5 1 A4 4 0 0 0 5 9" fill="none" stroke="currentColor" stroke-width="1.2" />
        </svg>
      );
    case 'dash':
      return (
        <svg class={`${styles.icon} ${colorClass}`} width={size} height={size} viewBox="0 0 10 10" aria-hidden="true">
          <line x1="2" y1="5" x2="8" y2="5" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
        </svg>
      );
  }
}

export default function StatusDot({ status, showLabel = true }: Props) {
  const label = STATUS_LABELS[status] || status;
  const colorClass = styles[status] || styles.pending;
  const shape = STATUS_SHAPES[status] || 'dash';

  return (
    <span class={styles.wrapper}>
      {/* a11y [WCAG 1.4.1]: shape + color convey status; label provides text alternative */}
      <span
        class={styles.shapeWrap}
        {...(showLabel ? { 'aria-hidden': 'true' } : { role: 'img', 'aria-label': label })}
      >
        <ShapeIcon shape={shape} colorClass={colorClass} />
      </span>
      {showLabel && <span class={styles.label}>{label}</span>}
    </span>
  );
}
