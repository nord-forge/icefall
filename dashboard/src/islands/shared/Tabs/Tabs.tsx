import { useState } from 'preact/hooks';
import styles from './tabs.module.css';
import type { ComponentChildren } from 'preact';

type Tab = {
  id: string;
  label: string;
  content: ComponentChildren;
  badge?: string;
};

type Props = {
  tabs: Tab[];
  defaultTab?: string;
};

export default function Tabs({ tabs, defaultTab }: Props) {
  const [active, setActive] = useState(defaultTab || tabs[0]?.id);
  const activeTab = tabs.find((t) => t.id === active);

  return (
    <div>
      <div class={styles.tabList} role="tablist">
        {tabs.map((tab) => (
          <button
            key={tab.id} type="button" role="tab"
            aria-selected={tab.id === active}
            aria-controls={`panel-${tab.id}`}
            class={`${styles.tab} ${tab.id === active ? styles.active : ''}`}
            onClick={() => setActive(tab.id)}
          >
            {tab.label}
            {tab.badge && <span class={styles.badge}>{tab.badge}</span>}
          </button>
        ))}
      </div>
      {activeTab && (
        <div id={`panel-${activeTab.id}`} role="tabpanel" aria-labelledby={activeTab.id} class={styles.panel}>
          {activeTab.content}
        </div>
      )}
    </div>
  );
}
