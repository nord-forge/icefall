import { useEffect, useState } from 'preact/hooks';
import type { Project, App } from '@lib/types';
import { api } from '@lib/api';
import { invalidatePrefix } from '@lib/cache';
import { formatRelativeTime } from '@lib/format';
import Button from '@islands/shared/Button/Button';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import { Plus, FolderOpen, Pencil, Trash2, ArrowLeft, Grid2x2, Database } from 'lucide-preact';
import { addToast } from '@stores/toast';
import { SkeletonCard } from '@islands/shared/Skeleton/Skeleton';
import Input from '@islands/shared/Input/Input';
import styles from './projects-page.module.css';
import formStyles from '@styles/form.module.css';

const PROJECT_COLORS = [
  'oklch(0.55 0.18 172.85)',
  'oklch(0.55 0.16 155)',
  'oklch(0.55 0.18 25)',
  'oklch(0.58 0.14 75)',
  'oklch(0.55 0.15 310)',
  'oklch(0.55 0.15 200)',
  'oklch(0.50 0.12 50)',
  'oklch(0.55 0.10 250)',
];

type ProjectDetail = Project & {
  apps: App[];
  databases: Array<{ id: string; name: string; db_type: string }>;
};

export default function ProjectsPage() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreate, setShowCreate] = useState(false);
  const [editingProject, setEditingProject] = useState<Project | null>(null);
  const [selectedProject, setSelectedProject] = useState<ProjectDetail | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState('');

  const [formName, setFormName] = useState('');
  const [formDescription, setFormDescription] = useState('');
  const [formColor, setFormColor] = useState(PROJECT_COLORS[0]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadProjects();
  }, []);

  async function loadProjects() {
    try {
      const { data } = await api.listProjects();
      setProjects(data);
    } catch {
      setError('Failed to load projects');
    }
    setLoading(false);
  }

  function openCreate() {
    setFormName('');
    setFormDescription('');
    setFormColor(PROJECT_COLORS[0]);
    setEditingProject(null);
    setShowCreate(true);
    setError('');
  }

  function openEdit(project: Project, e: Event) {
    e.stopPropagation();
    setFormName(project.name);
    setFormDescription(project.description || '');
    setFormColor(project.color || PROJECT_COLORS[0]);
    setEditingProject(project);
    setShowCreate(true);
    setError('');
  }

  function closeForm() {
    setShowCreate(false);
    setEditingProject(null);
    setError('');
  }

  async function handleSave() {
    if (!formName.trim()) {
      setError('Project name is required');
      return;
    }
    setSaving(true);
    setError('');
    try {
      if (editingProject) {
        await api.updateProject(editingProject.id, {
          name: formName.trim(),
          description: formDescription.trim() || null,
          color: formColor,
        });
      } else {
        await api.createProject({
          name: formName.trim(),
          description: formDescription.trim() || undefined,
          color: formColor,
        });
      }
      closeForm();
      await loadProjects();
    } catch (err: any) {
      setError(err.message || 'Failed to save project');
    }
    setSaving(false);
  }

  async function handleDelete(id: string) {
    try {
      await api.deleteProject(id);
      if (selectedProject?.id === id) {
        setSelectedProject(null);
      }
      await loadProjects();
    } catch (err: any) {
      setError(err.message || 'Failed to delete project');
    }
  }

  async function openDetail(project: Project, pushState = true) {
    try {
      const { data } = await api.getProject(project.id);
      setSelectedProject(data as ProjectDetail);
      if (pushState) {
        window.history.pushState({ projectId: project.id }, '', `/projects?id=${project.id}`);
      }
    } catch {
      setError('Failed to load project details');
    }
  }

  function backToList() {
    setSelectedProject(null);
    setError('');
    window.history.pushState(null, '', '/projects');
  }

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const id = params.get('id');
    if (id && projects.length > 0) {
      const project = projects.find((p) => p.id === id);
      if (project) openDetail(project, false);
    }

    function handlePopState() {
      const params = new URLSearchParams(window.location.search);
      const id = params.get('id');
      if (id) {
        const project = projects.find((p) => p.id === id);
        if (project) openDetail(project, false);
      } else {
        setSelectedProject(null);
      }
    }

    window.addEventListener('popstate', handlePopState);
    return () => window.removeEventListener('popstate', handlePopState);
  }, [projects]);

  // Detail view state
  const [showAddDb, setShowAddDb] = useState(false);
  const [newDbName, setNewDbName] = useState('');
  const [newDbType, setNewDbType] = useState('postgres');
  const [creatingDb, setCreatingDb] = useState(false);

  async function handleCreateDb() {
    if (!newDbName.trim() || !selectedProject) return;
    setCreatingDb(true);
    try {
      await api.createDatabase({ name: newDbName.trim(), db_type: newDbType, app_id: undefined });
      addToast('success', `Database "${newDbName.trim()}" created`);
      setNewDbName('');
      setShowAddDb(false);
      invalidatePrefix('/projects');
      invalidatePrefix('/databases');
      const { data } = await api.getProject(selectedProject.id);
      setSelectedProject(data as ProjectDetail);
    } catch (err: any) {
      addToast('error', err.message || 'Failed to create database');
    }
    setCreatingDb(false);
  }

  // Detail view
  if (selectedProject) {
    return (
      <div>
        <button
          type="button"
          class={styles.backButton}
          onClick={backToList}
        >
          <ArrowLeft size={14} style={{ verticalAlign: 'middle', marginRight: '4px' }} />
          All Projects
        </button>

        <div class={styles.detailHeader}>
          {selectedProject.color && (
            <span
              class={styles.projectColor}
              style={{ background: selectedProject.color, width: '16px', height: '16px' }}
              aria-hidden="true"
            />
          )}
          <div>
            <h1 class={styles.detailTitle}>{selectedProject.name}</h1>
            {selectedProject.description && (
              <p class={styles.detailDescription}>{selectedProject.description}</p>
            )}
          </div>
        </div>

        <section aria-labelledby="project-apps-heading" style={{ marginBottom: 'var(--space-6)' }}>
          <div class={styles.sectionHeader}>
            <h2 id="project-apps-heading" class={styles.sectionTitle}>
              Apps ({selectedProject.apps?.length || 0})
            </h2>
            <a href={`/apps/new?project_id=${selectedProject.id}`}>
              <Button variant="secondary" size="sm">
                <Plus size={14} /> Add app
              </Button>
            </a>
          </div>

          {selectedProject.apps && selectedProject.apps.length > 0 ? (
            <div class={styles.resourceList}>
              {selectedProject.apps.map((app) => (
                <a key={app.id} href={`/apps/${app.id}`} class={styles.resourceCard}>
                  <Grid2x2 size={16} aria-hidden="true" />
                  <div>
                    <div class={styles.resourceName}>{app.name}</div>
                    <div class={styles.resourceType}>
                      {app.framework || 'App'} &middot; {app.git_branch}
                    </div>
                  </div>
                </a>
              ))}
            </div>
          ) : (
            <p class={styles.noResources}>No apps in this project yet. Click "Add app" to get started.</p>
          )}
        </section>

        <section aria-labelledby="project-dbs-heading">
          <div class={styles.sectionHeader}>
            <h2 id="project-dbs-heading" class={styles.sectionTitle}>
              Databases ({selectedProject.databases?.length || 0})
            </h2>
            {!showAddDb && (
              <Button variant="secondary" size="sm" onClick={() => setShowAddDb(true)}>
                <Plus size={14} /> Add database
              </Button>
            )}
          </div>

          {showAddDb && (
            <div class={styles.addResourceForm}>
              <Input
                label="Database name"
                name="new-db-name"
                value={newDbName}
                onChange={setNewDbName}
                placeholder="my-database"
              />
              <div>
                <label class={formStyles.label} style={{ marginBottom: 'var(--space-2)', display: 'block' }}>Type</label>
                <div class={styles.dbTypeOptions} role="radiogroup" aria-label="Database type">
                  {['postgres', 'mysql', 'redis', 'mongo'].map((t) => (
                    <button
                      key={t}
                      type="button"
                      class={`${styles.dbTypeOption} ${newDbType === t ? styles.dbTypeOptionSelected : ''}`}
                      onClick={() => setNewDbType(t)}
                      role="radio"
                      aria-checked={newDbType === t}
                    >
                      {t === 'postgres' ? 'PostgreSQL' : t === 'mysql' ? 'MySQL' : t === 'redis' ? 'Redis' : 'MongoDB'}
                    </button>
                  ))}
                </div>
              </div>
              <div class={styles.addResourceActions}>
                <Button variant="ghost" size="sm" onClick={() => { setShowAddDb(false); setNewDbName(''); }}>
                  Cancel
                </Button>
                <Button variant="primary" size="sm" onClick={handleCreateDb} loading={creatingDb} disabled={!newDbName.trim()}>
                  Create database
                </Button>
              </div>
            </div>
          )}

          {selectedProject.databases && selectedProject.databases.length > 0 ? (
            <div class={styles.resourceList}>
              {selectedProject.databases.map((db) => (
                <a key={db.id} href="/databases" class={styles.resourceCard}>
                  <Database size={16} aria-hidden="true" />
                  <div>
                    <div class={styles.resourceName}>{db.name}</div>
                    <div class={styles.resourceType}>{db.db_type}</div>
                  </div>
                </a>
              ))}
            </div>
          ) : !showAddDb ? (
            <p class={styles.noResources}>No databases in this project yet. Click "Add database" to get started.</p>
          ) : null}
        </section>
      </div>
    );
  }

  // List view
  return (
    <div>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Projects</h1>
        <Button variant="primary" onClick={openCreate}>
          <Plus size={14} /> New Project
        </Button>
      </div>

      {error && <p class={styles.errorMessage} role="alert">{error}</p>}

      {showCreate && (
        <div class={styles.formCard}>
          <h2 class={styles.formTitle}>
            {editingProject ? 'Edit Project' : 'New Project'}
          </h2>
          <div class={formStyles.fieldGroup}>
            <Input
              label="Name"
              name="project-name"
              id="project-name"
              value={formName}
              onChange={setFormName}
              placeholder="My Project"
            />
            <Input
              label="Description"
              name="project-description"
              id="project-description"
              value={formDescription}
              onChange={setFormDescription}
              placeholder="Optional description"
            />
            <div>
              {/* a11y [WCAG 1.3.1]: group label for color selection */}
              <fieldset style={{ border: 'none', padding: 0, margin: 0 }}>
                <legend class={formStyles.label} style={{ marginBottom: 'var(--space-2)' }}>Color</legend>
                <div class={styles.colorOptions} role="radiogroup">
                  {PROJECT_COLORS.map((color) => (
                    <button
                      key={color}
                      type="button"
                      class={`${styles.colorSwatch} ${formColor === color ? styles.colorSwatchSelected : ''}`}
                      style={{ background: color }}
                      onClick={() => setFormColor(color)}
                      role="radio"
                      aria-checked={formColor === color}
                      aria-label={`Color ${color}`}
                    />
                  ))}
                </div>
              </fieldset>
            </div>
          </div>
          <div class={styles.formActions}>
            <Button variant="ghost" onClick={closeForm}>Cancel</Button>
            <Button variant="primary" onClick={handleSave} loading={saving}>
              {editingProject ? 'Update' : 'Create'}
            </Button>
          </div>
        </div>
      )}

      {loading ? (
        <div class={styles.projectGrid}>
          <SkeletonCard />
          <SkeletonCard />
          <SkeletonCard />
        </div>
      ) : projects.length === 0 && !showCreate ? (
        <div class={styles.emptyState}>
          <div class={styles.emptyIcon}>
            <FolderOpen size={48} />
          </div>
          <p class={styles.emptyTitle}>No projects yet</p>
          <p class={styles.emptyHint}>
            Projects group related apps and databases together.
          </p>
        </div>
      ) : (
        <div class={styles.projectGrid}>
          {projects.map((project) => (
            <button
              key={project.id}
              type="button"
              class={styles.projectCard}
              onClick={() => openDetail(project)}
            >
              <div class={styles.projectCardHeader}>
                {project.color && (
                  <span
                    class={styles.projectColor}
                    style={{ background: project.color }}
                    aria-hidden="true"
                  />
                )}
                <span class={styles.projectName}>{project.name}</span>
                <div class={styles.projectActions}>
                  {/* a11y [WCAG 4.1.2]: buttons have accessible names via aria-label */}
                  <button
                    type="button"
                    class={styles.iconButton}
                    onClick={(e: Event) => openEdit(project, e)}
                    aria-label={`Edit project ${project.name}`}
                  >
                    <Pencil size={14} />
                  </button>
                  <button
                    type="button"
                    class={`${styles.iconButton} ${styles.iconButtonDanger}`}
                    onClick={(e: Event) => { e.stopPropagation(); setConfirmDeleteId(project.id); }}
                    aria-label={`Delete project ${project.name}`}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
              {project.description && (
                <p class={styles.projectDescription}>{project.description}</p>
              )}
              <div class={styles.projectMeta}>
                <span>{project.app_count ?? 0} apps</span>
                <span>{project.database_count ?? 0} databases</span>
                <span>{formatRelativeTime(project.created_at)}</span>
              </div>
            </button>
          ))}
        </div>
      )}

      <ConfirmDialog
        open={confirmDeleteId !== null}
        title="Delete project?"
        description={`This will permanently remove "${projects.find((p) => p.id === confirmDeleteId)?.name ?? 'this project'}" and unlink all its apps and databases. This action cannot be undone.`}
        confirmLabel="Delete"
        variant="danger"
        loading={deleting}
        onConfirm={async () => {
          if (!confirmDeleteId) return;
          setDeleting(true);
          try {
            await handleDelete(confirmDeleteId);
          } finally {
            setDeleting(false);
            setConfirmDeleteId(null);
          }
        }}
        onCancel={() => setConfirmDeleteId(null)}
      />
    </div>
  );
}
