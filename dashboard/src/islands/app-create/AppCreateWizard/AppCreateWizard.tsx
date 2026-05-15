import { useState, useRef, useEffect, useMemo } from 'preact/hooks';
import { api } from '@lib/api';
import type { Server } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import ServerSelectStep from '@islands/app-create/ServerSelectStep/ServerSelectStep';
import { ArrowLeft, ArrowRight, Rocket } from 'lucide-preact';
import SourceCards from './components/SourceCards';
import OneClickServices from './components/OneClickServices';
import type { OneClickService } from './components/OneClickServices';
import GitRepoStep from './components/GitRepoStep';
import BuildSettingsStep from './components/BuildSettingsStep';
import ImageStep from './components/ImageStep';
import ComposeStep from './components/ComposeStep';
import EnvStep from './components/EnvStep';
import ReviewStep from './components/ReviewStep';
import styles from './app-create.module.css';

type DeploySource = 'git' | 'image' | 'compose';

export default function AppCreateWizard() {
  const [step, setStep] = useState(0);
  const [deploySource, setDeploySource] = useState<DeploySource | null>(null);
  const [deploying, setDeploying] = useState(false);
  const [deployingService, setDeployingService] = useState<string | null>(null);
  const [composeError, setComposeError] = useState('');
  const [servers, setServers] = useState<Server[]>([]);
  const [selectedServerId, setSelectedServerId] = useState<string | null>(null);
  const hasMultipleServers = servers.length >= 2;
  const [form, setForm] = useState({
    name: '',
    git_repo: '',
    git_branch: 'main',
    token: '',
    build_command: '',
    output_dir: '',
    start_command: '',
    port: '3000',
    envContent: '',
    image_ref: '',
    compose_content: '',
  });

  const projectId = useMemo(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get('project_id') || null;
  }, []);

  useEffect(() => {
    api.listServers().then(({ data }) => {
      setServers(data);
      const recommended = data.find((s) => s.recommended && s.status === 'online');
      if (recommended) setSelectedServerId(recommended.id);
      else {
        const firstOnline = data.find((s) => s.status === 'online' && s.role !== 'control-plane') || data.find((s) => s.status === 'online');
        if (firstOnline) setSelectedServerId(firstOnline.id);
      }
    }).catch(() => {});
  }, []);

  const steps = useMemo(() => {
    if (!deploySource) return ['Source'];
    const base = deploySource === 'git'
      ? ['Source', 'Repository', 'Build Settings']
      : deploySource === 'compose'
        ? ['Source', 'Compose File']
        : ['Source', 'Docker Image'];
    if (hasMultipleServers) base.push('Server');
    base.push('Environment', 'Review');
    return base;
  }, [deploySource, hasMultipleServers]);

  const lastStep = steps.length - 1;
  const isReviewStep = step === lastStep;
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

  function update(field: string, value: string) {
    // Clear validation error for this field when user types
    setValidationErrors(prev => {
      const next = { ...prev };
      delete next[field];
      return next;
    });
    setForm((prev) => ({ ...prev, [field]: value }));
    if (field === 'git_repo' && !form.name) {
      const name = value.split('/').pop()?.replace('.git', '') || '';
      setForm((prev) => ({ ...prev, name }));
    }
    if (field === 'image_ref' && !form.name) {
      // Derive app name from image ref: "ghost:5-alpine" -> "ghost", "plausible/analytics:v2" -> "analytics"
      const name = value.split(':')[0]?.split('/').pop() || '';
      setForm((prev) => ({ ...prev, name }));
    }
    if (field === 'compose_content') {
      setComposeError('');
    }
  }

  function handleSourceSelect(source: DeploySource) {
    setDeploySource(source);
    setStep(1);
  }

  // a11y [WCAG 2.4.3]: move focus to step card on step change
  const cardRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    cardRef.current?.focus();
  }, [step]);

  const currentStepName = steps[step] || '';

  function canAdvance(): boolean {
    if (step === 0) return false;
    if (currentStepName === 'Repository') return !!form.name.trim();
    if (currentStepName === 'Docker Image') return !!form.name.trim() && !!form.image_ref.trim() && !!form.port.trim();
    if (currentStepName === 'Compose File') return !!form.name.trim() && !!form.compose_content.trim();
    if (currentStepName === 'Server') return !!selectedServerId;
    return true;
  }

  function validateAndAdvance() {
    const errors: Record<string, string> = {};

    if (currentStepName === 'Repository') {
      if (!form.name.trim()) errors.name = 'App name is required';
    }
    if (currentStepName === 'Docker Image') {
      if (!form.name.trim()) errors.name = 'App name is required';
      if (!form.image_ref.trim()) errors.image_ref = 'Docker image is required';
      if (!form.port.trim()) errors.port = 'Container port is required';
    }
    if (currentStepName === 'Compose File') {
      if (!form.name.trim()) errors.name = 'Stack name is required';
      if (!form.compose_content.trim()) errors.compose_content = 'Compose file content is required';
    }
    if (currentStepName === 'Server' && !selectedServerId) {
      errors.server = 'Select a server';
    }

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    setValidationErrors({});
    setStep(step + 1);
  }

  async function handleDeploy() {
    setDeploying(true);
    try {
      const isImage = deploySource === 'image';
      const isCompose = deploySource === 'compose';

      const createBody: Parameters<typeof api.createApp>[0] = {
        name: form.name,
      };

      if (isCompose) {
        createBody.compose_content = form.compose_content;
      } else if (isImage) {
        createBody.image_ref = form.image_ref;
        createBody.port = parseInt(form.port, 10) || 3000;
      } else {
        createBody.git_repo = form.git_repo || undefined;
        createBody.git_branch = form.git_branch;
      }

      if (hasMultipleServers && selectedServerId) {
        createBody.server_id = selectedServerId;
      }

      const { data: app } = await api.createApp(createBody);

      if (projectId) {
        await api.updateApp(app.id, { project_id: projectId } as any);
      }

      if (form.envContent.trim()) {
        await api.importEnv(app.id, form.envContent, 'shared');
      }

      const { data: deploy } = await api.triggerDeploy(app.id);
      window.location.href = `/apps/${app.id}/deploys/${deploy.id}`;
    } catch (err: any) {
      if (err?.message?.includes('Compose YAML')) {
        setComposeError(err.message);
      }
      setDeploying(false);
    }
  }

  function handleBack() {
    setValidationErrors({});
    if (step === 1) {
      // Going back to source selection
      setDeploySource(null);
      setStep(0);
    } else {
      setStep(step - 1);
    }
  }

  async function handleOneClickDeploy(service: OneClickService) {
    setDeployingService(service.name);
    try {
      const name = service.name.toLowerCase().replace(/\s+/g, '-');
      const createBody: Parameters<typeof api.createApp>[0] = { name };

      if (service.compose_content) {
        createBody.compose_content = service.compose_content;
      }

      const recommended = servers.find((s) => s.recommended && s.status === 'online');
      const firstOnline = servers.find((s) => s.status === 'online' && s.role !== 'control-plane') || servers.find((s) => s.status === 'online');
      const targetServer = recommended || firstOnline;
      if (targetServer && hasMultipleServers) {
        createBody.server_id = targetServer.id;
      }

      const { data: app } = await api.createApp(createBody);

      if (projectId) {
        await api.updateApp(app.id, { project_id: projectId } as any);
      }

      if (service.default_env) {
        try {
          const envObj = JSON.parse(service.default_env);
          const envContent = Object.entries(envObj)
            .map(([k, v]) => `${k}=${v}`)
            .join('\n');
          if (envContent) await api.importEnv(app.id, envContent, 'shared');
        } catch { /* invalid JSON, skip */ }
      }

      const { data: deploy } = await api.triggerDeploy(app.id);
      window.location.href = `/apps/${app.id}/deploys/${deploy.id}`;
    } catch {
      setDeployingService(null);
    }
  }

  // --- Determine which content to render for the current step ---

  function renderStepContent() {
    switch (currentStepName) {
      case 'Repository':
        return (
          <GitRepoStep
            name={form.name}
            gitRepo={form.git_repo}
            gitBranch={form.git_branch}
            validationErrors={validationErrors}
            onUpdate={update}
          />
        );
      case 'Build Settings':
        return (
          <BuildSettingsStep
            buildCommand={form.build_command}
            outputDir={form.output_dir}
            startCommand={form.start_command}
            port={form.port}
            onUpdate={update}
          />
        );
      case 'Docker Image':
        return (
          <ImageStep
            name={form.name}
            imageRef={form.image_ref}
            port={form.port}
            validationErrors={validationErrors}
            onUpdate={update}
          />
        );
      case 'Compose File':
        return (
          <ComposeStep
            name={form.name}
            composeContent={form.compose_content}
            composeError={composeError}
            validationErrors={validationErrors}
            onUpdate={update}
          />
        );
      case 'Server':
        return (
          <ServerSelectStep
            servers={servers}
            selectedId={selectedServerId}
            onSelect={setSelectedServerId}
          />
        );
      case 'Environment':
        return (
          <EnvStep
            envContent={form.envContent}
            deploySource={deploySource}
            onUpdate={update}
          />
        );
      case 'Review':
        return (
          <ReviewStep
            deploySource={deploySource}
            form={form}
            hasMultipleServers={hasMultipleServers}
            selectedServerId={selectedServerId}
            servers={servers}
          />
        );
      default:
        return null;
    }
  }

  return (
    <div class={styles.wrapper}>
      <h1 class={styles.pageTitle}>
        Create New App
      </h1>

      {/* a11y [WCAG 1.3.1]: progress indicator with aria-current — hidden on single-step view */}
      {steps.length > 1 && (
        <nav aria-label="Progress" class={styles.progress}>
          {steps.map((s, i) => (
            <div key={s} class={styles.progressStep} aria-current={i === step ? 'step' : undefined}>
              <div class={i <= step ? styles.progressBarActive : styles.progressBarInactive} />
              <span class={i === step ? styles.progressLabelCurrent : i < step ? styles.progressLabelActive : styles.progressLabelInactive}>
                {s}
              </span>
            </div>
          ))}
        </nav>
      )}

      {step === 0 ? (
        <>
          <div ref={cardRef} tabIndex={-1} class={styles.card}>
            <SourceCards onSelect={handleSourceSelect} />
          </div>
          <OneClickServices
            deployingService={deployingService}
            onDeploy={handleOneClickDeploy}
          />
          <div class={styles.navigationEnd}>
            <a href="/" class={styles.cancelLink}><Button variant="ghost">Cancel</Button></a>
          </div>
        </>
      ) : (
        <>
          <div ref={cardRef} tabIndex={-1} class={styles.card}>
            {renderStepContent()}
          </div>
          <div class={styles.navigation}>
            <Button variant="ghost" onClick={handleBack}>
              <ArrowLeft size={14} /> Back
            </Button>
            {!isReviewStep ? (
              <Button variant="primary" onClick={validateAndAdvance}>
                Next <ArrowRight size={14} />
              </Button>
            ) : (
              <Button variant="primary" onClick={handleDeploy} loading={deploying} disabled={!form.name.trim()}>
                <Rocket size={14} /> Deploy
              </Button>
            )}
          </div>
        </>
      )}
    </div>
  );
}
