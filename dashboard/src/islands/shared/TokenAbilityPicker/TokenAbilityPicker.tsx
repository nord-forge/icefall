import Button from '@islands/shared/Button/Button';
import styles from './token-ability-picker.module.css';

// IF-168: granular API token ability scoping. Single source of truth for the
// ability list, presets, picker, and badges — used by both the profile and
// admin token UIs so they never drift apart.

// Keep in sync with ALL_ABILITIES in src/api/abilities.rs.
export const ALL_ABILITIES = [
  'apps:read', 'apps:write', 'apps:deploy',
  'databases:read', 'databases:write',
  'domains:read', 'domains:write',
  'env:read', 'env:write',
  'servers:read', 'servers:write',
  'users:read', 'users:write',
  'settings:read', 'settings:write',
];

// An empty scope list means "full access" (null abilities server-side).
export const ABILITY_PRESETS: Record<string, string[]> = {
  'Full access': [],
  'Read only': ALL_ABILITIES.filter(a => a.endsWith(':read')),
  'Deploy only': ['apps:read', 'apps:deploy', 'env:read'],
};

type Props = {
  /** Currently-selected scopes. Empty array = full access. */
  value: string[];
  onChange: (abilities: string[]) => void;
};

/** Fieldset with preset buttons + a checkbox grid of every ability scope. */
export default function TokenAbilityPicker({ value, onChange }: Props) {
  function toggle(ability: string) {
    onChange(
      value.includes(ability) ? value.filter(a => a !== ability) : [...value, ability],
    );
  }

  return (
    // a11y [WCAG 1.3.1]: related controls grouped under a legend
    <fieldset class={styles.fieldset}>
      <legend class={styles.legend}>Abilities</legend>
      <p class={styles.hint}>
        Leave all unchecked for full access. Pick a preset or choose scopes.
      </p>
      <div class={styles.presets}>
        {Object.entries(ABILITY_PRESETS).map(([label, scopes]) => (
          <Button key={label} variant="ghost" size="sm" onClick={() => onChange([...scopes])}>
            {label}
          </Button>
        ))}
      </div>
      <div class={styles.grid}>
        {ALL_ABILITIES.map(ability => (
          <label key={ability} class={styles.checkbox}>
            <input
              type="checkbox"
              checked={value.includes(ability)}
              onChange={() => toggle(ability)}
            />
            <span>{ability}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/** Read-only badge list for a token's granted abilities, or a "Full access" tag. */
export function TokenAbilityBadges({ abilities }: { abilities?: string[] }) {
  if (!abilities || abilities.length === 0) {
    return <span class={styles.fullAccess}>Full access</span>;
  }
  return (
    <span class={styles.badges}>
      {abilities.map(a => (
        <span key={a} class={styles.badge}>{a}</span>
      ))}
    </span>
  );
}
