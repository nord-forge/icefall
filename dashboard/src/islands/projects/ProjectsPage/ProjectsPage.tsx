import { useEffect, useState } from 'preact/hooks';
import type { Project, App } from '@lib/types';
import { api } from '@lib/api';
import { formatRelativeTime } from '@lib/format';
import Button from '@islands/shared/Button/Button';
import { Plus, FolderOpen, Pencil, Trash2, ArrowLeft, Grid2x2, Database } from 'lucide-preact';
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

  async function handleDelete(id: string, e: Event) {
    e.stopPropagation();
    if (confirmDeleteId !== id) {
      setConfirmDeleteId(id);
      return;
    }
    try {
      await api.deleteProject(id);
      setConfirmDeleteId(null);
      if (selectedProject?.id === id) {
        setSelectedProject(null);
      }
      await loadProjects();
    } catch (err: any) {
      setError(err.message || 'Failed to delete project');
    }
  }

  async function openDetail(project: Project) {
    try {
      const { data } = await api.getProject(project.id);
      setSelectedProject(data as ProjectDetail);
    } catch {
      setError('Failed to load project details');
    }
  }

  function backToList() {
    setSelectedProject(null);
    setError('');
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
          <h2 id="project-apps-heading" class={styles.sectionTitle}>
            Apps ({selectedProject.apps?.length || 0})
          </h2>
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
            <p class={styles.noResources}>No apps assigned to this project yet.</p>
          )}
        </section>

        <section aria-labelledby="project-dbs-heading">
          <h2 id="project-dbs-heading" class={styles.sectionTitle}>
            Databases ({selectedProject.databases?.length || 0})
          </h2>
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
          ) : (
            <p class={styles.noResources}>No databases assigned to this project yet.</p>
          )}
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
                  {confirmDeleteId === project.id ? (
                    <div class={styles.confirmRow}>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e: Event) => { e.stopPropagation(); setConfirmDeleteId(null); }}
                      >
                        Cancel
                      </Button>
                      <Button
                        variant="danger"
                        size="sm"
                        onClick={(e: Event) => handleDelete(project.id, e)}
                      >
                        <Trash2 size={12} /> Delete
                      </Button>
                    </div>
                  ) : (
                    <button
                      type="button"
                      class={`${styles.iconButton} ${styles.iconButtonDanger}`}
                      onClick={(e: Event) => handleDelete(project.id, e)}
                      aria-label={`Delete project ${project.name}`}
                    >
                      <Trash2 size={14} />
                    </button>
                  )}
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
    </div>
  );
}
