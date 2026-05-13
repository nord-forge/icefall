import { X } from 'lucide-preact';
import styles from '../settings-tab.module.css';

type Props = {
  tags: string[];
  tagInput: string;
  tagError: string;
  tagMaxLength: number;
  onTagInputChange: (v: string) => void;
  onAddTag: (raw: string) => void;
  onRemoveTag: (tag: string) => void;
  onTagKeyDown: (e: KeyboardEvent) => void;
};

export default function TagsCard({
  tags,
  tagInput,
  tagError,
  tagMaxLength,
  onTagInputChange,
  onAddTag,
  onRemoveTag,
  onTagKeyDown,
}: Props) {
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>Tags</h2>
      <p class={styles.settingsDescription}>
        Organize apps with freeform tags. Tags are lowercase, alphanumeric with hyphens, max {tagMaxLength} characters.
      </p>
      <div class={styles.tagInputWrap}>
        {tags.map((tag) => (
          <span key={tag} class={styles.tagChip}>
            {tag}
            {/* a11y [WCAG 4.1.2]: button has accessible name via aria-label */}
            <button
              type="button"
              class={styles.tagRemove}
              onClick={() => onRemoveTag(tag)}
              aria-label={`Remove tag ${tag}`}
            >
              <X size={12} />
            </button>
          </span>
        ))}
        <input
          id="settings-tags"
          class={styles.tagInputField}
          type="text"
          value={tagInput}
          onInput={(e) => {
            const val = (e.target as HTMLInputElement).value;
            if (val.includes(',')) {
              onAddTag(val.replace(',', ''));
            } else {
              onTagInputChange(val);
            }
          }}
          onKeyDown={onTagKeyDown}
          placeholder={tags.length === 0 ? 'Add a tag...' : ''}
          aria-label="Add tag"
          maxLength={tagMaxLength}
        />
      </div>
      {tagError && (
        <p class={styles.tagError} role="alert">{tagError}</p>
      )}
      <span class={styles.fieldHint}>Press Enter or comma to add a tag.</span>
    </div>
  );
}
