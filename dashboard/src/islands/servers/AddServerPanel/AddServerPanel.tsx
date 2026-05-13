import { useState, useEffect, useRef } from 'preact/hooks';
import { api } from '@lib/api';
import { createSSEClient } from '@lib/sse';
import type { Server } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import { X, Copy, Check, RefreshCw, ExternalLink, Loader } from 'lucide-preact';
import formStyles from '@styles/form.module.css';
import styles from './add-server-panel.module.css';

type Props = {
  onClose: () => void;
  onServerAdded: (server: Server) => void;
};

type EnrollmentStep = {
  label: string;
  status: 'pending' | 'active' | 'done' | 'failed';
};

export default function AddServerPanel({ onClose, onServerAdded }: Props) {
  const [name, setName] = useState('');
  const [host, setHost] = useState('');
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState('');
  const [server, setServer] = useState<(Server & { enrollment_token: string }) | null>(null);
  const [copied, setCopied] = useState(false);
  const [tokenCreatedAt, setTokenCreatedAt] = useState<number | null>(null);
  const [timeLeft, setTimeLeft] = useState('');
  const [enrollmentDone, setEnrollmentDone] = useState(false);
  const [enrollmentSteps, setEnrollmentSteps] = useState<EnrollmentStep[]>([
    { label: 'Agent connected', status: 'pending' },
    { label: 'Docker check passed', status: 'pending' },
    { label: 'Network verified', status: 'pending' },
    { label: 'Server registered', status: 'pending' },
  ]);

  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    nameRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!tokenCreatedAt) return;
    const TOKEN_TTL = 15 * 60 * 1000;

    function tick() {
      const elapsed = Date.now() - tokenCreatedAt!;
      const remaining = Math.max(0, TOKEN_TTL - elapsed);
      if (remaining <= 0) {
        setTimeLeft('Expired');
        return;
      }
      const mins = Math.floor(remaining / 60000);
      const secs = Math.floor((remaining % 60000) / 1000);
      setTimeLeft(`${mins}:${secs.toString().padStart(2, '0')}`);
    }

    tick();
    const interval = setInterval(tick, 1000);
    return () => clearInterval(interval);
  }, [tokenCreatedAt]);

  useEffect(() => {
    if (!server) return;

    const sse = createSSEClient('/api/v1/events', {
      'server.connected': (data: any) => {
        if (data.server_id !== server.id) return;
        updateStep(0, 'done');
        updateStep(1, 'active');
      },
      'server.enrollment.step': (data: any) => {
        if (data.server_id !== server.id) return;
        if (data.step === 'docker_ok') {
          updateStep(1, 'done');
          updateStep(2, 'active');
        }
        if (data.step === 'network_ok') {
          updateStep(2, 'done');
          updateStep(3, 'active');
        }
        if (data.step === 'registered') {
          updateStep(3, 'done');
          setEnrollmentDone(true);
          onServerAdded({ ...server, status: 'online' });
        }
        if (data.step === 'failed') {
          const failIdx = enrollmentSteps.findIndex((s) => s.status === 'active');
          if (failIdx >= 0) updateStep(failIdx, 'failed');
        }
      },
    });

    updateStep(0, 'active');

    return () => sse.close();
  }, [server?.id]);

  function updateStep(index: number, status: EnrollmentStep['status']) {
    setEnrollmentSteps((prev) =>
      prev.map((s, i) => (i === index ? { ...s, status } : s))
    );
  }

  async function handleGenerate() {
    if (!name.trim() || !host.trim()) return;
    setCreating(true);
    setError('');
    try {
      const { data } = await api.createServer({ name: name.trim(), host: host.trim() });
      setServer(data);
      setTokenCreatedAt(Date.now());
    } catch (err: any) {
      setError(err.message || 'Failed to create server');
    }
    setCreating(false);
  }

  async function handleRegenerateToken() {
    if (!server) return;
    try {
      const { data } = await api.regenerateServerToken(server.id);
      setServer({ ...server, enrollment_token: data.enrollment_token });
      setTokenCreatedAt(Date.now());
    } catch (err: any) {
      setError(err.message || 'Failed to regenerate token');
    }
  }

  function handleCopy() {
    if (!server) return;
    const command = `curl -fsSL ${window.location.origin}/api/v1/servers/setup | bash -s -- --token ${server.enrollment_token}`;
    navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  const stepIcons = {
    pending: <span class={styles.stepDotPending} aria-hidden="true" />,
    active: <Loader size={14} class={styles.stepSpinner} aria-hidden="true" />,
    done: <Check size={14} class={styles.stepDotDone} aria-hidden="true" />,
    failed: <X size={14} class={styles.stepDotFailed} aria-hidden="true" />,
  };

  return (
    <div class={styles.panel} role="region" aria-label="Add server" onKeyDown={handleKeyDown}>
      <div class={styles.panelHeader}>
        <h2 class={styles.panelTitle}>Add a server</h2>
        <button type="button" class={styles.closeButton} onClick={onClose} aria-label="Close panel">
          <X size={16} />
        </button>
      </div>

      {!server && (
        <div class={styles.panelBody}>
          <div class={formStyles.fieldGroup}>
            <div>
              <label htmlFor="server-name" class={formStyles.label}>Server name</label>
              <input
                ref={nameRef}
                id="server-name"
                class={formStyles.input}
                value={name}
                onInput={(e) => setName((e.target as HTMLInputElement).value)}
                placeholder="production-worker-01"
              />
            </div>
            <div>
              <label htmlFor="server-host" class={formStyles.label}>Host IP or hostname</label>
              <input
                id="server-host"
                class={formStyles.inputMono}
                value={host}
                onInput={(e) => setHost((e.target as HTMLInputElement).value)}
                placeholder="203.0.113.42"
              />
            </div>
          </div>

          {error && <p role="alert" class={styles.error}>{error}</p>}

          <div class={styles.panelActions}>
            <Button
              variant="primary"
              onClick={handleGenerate}
              loading={creating}
              disabled={!name.trim() || !host.trim()}
            >
              Generate setup command
            </Button>
          </div>
        </div>
      )}

      {server && !enrollmentDone && (
        <div class={styles.panelBody}>
          <div class={styles.commandSection}>
            <label class={formStyles.label}>Run this command on your server</label>
            <div class={styles.commandBox}>
              <code class={styles.command}>
                curl -fsSL {window.location.origin}/api/v1/servers/setup | bash -s -- --token {server.enrollment_token}
              </code>
              <button type="button" class={styles.copyButton} onClick={handleCopy} aria-label="Copy command">
                {copied ? <Check size={14} /> : <Copy size={14} />}
              </button>
            </div>
            <div class={styles.tokenMeta}>
              <span class={styles.tokenExpiry}>
                {timeLeft === 'Expired' ? 'Token expired' : `Token expires in ${timeLeft}`}
              </span>
              {timeLeft === 'Expired' && (
                <button type="button" class={styles.regenerateLink} onClick={handleRegenerateToken}>
                  <RefreshCw size={12} /> Regenerate
                </button>
              )}
            </div>
          </div>

          <div class={styles.enrollmentProgress}>
            <h3 class={styles.progressTitle}>Connection progress</h3>
            <ol class={styles.stepList} aria-label="Enrollment steps">
              {enrollmentSteps.map((s, i) => (
                <li key={i} class={styles.stepItem} aria-current={s.status === 'active' ? 'step' : undefined}>
                  {stepIcons[s.status]}
                  <span class={s.status === 'done' ? styles.stepLabelDone : s.status === 'failed' ? styles.stepLabelFailed : styles.stepLabel}>
                    {s.label}
                  </span>
                </li>
              ))}
            </ol>
          </div>

          {error && <p role="alert" class={styles.error}>{error}</p>}
        </div>
      )}

      {enrollmentDone && server && (
        <div class={styles.panelBody}>
          <div class={styles.successState}>
            <Check size={24} class={styles.successIcon} />
            <p class={styles.successTitle}>Server ready to receive deployments</p>
            <a href={`/servers/${server.id}`}>
              <Button variant="primary">
                <ExternalLink size={14} /> View server
              </Button>
            </a>
          </div>
        </div>
      )}
    </div>
  );
}
