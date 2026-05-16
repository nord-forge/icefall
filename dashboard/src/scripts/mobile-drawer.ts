function initMobileDrawer() {
  const toggle = document.getElementById('sidebar-toggle');
  const backdrop = document.getElementById('sidebar-backdrop');
  const panel = document.getElementById('sidebar-panel');
  const sidebar = panel?.querySelector<HTMLElement>('.ice-sidebar');
  if (!toggle || !backdrop || !sidebar) return;

  function openDrawer() {
    sidebar!.classList.add('_sidebarOpen');
    backdrop!.classList.add('_backdropVisible');
    toggle!.setAttribute('aria-expanded', 'true');
    toggle!.setAttribute('aria-label', 'Close navigation');
    document.addEventListener('keydown', trapFocus);
    const firstLink = sidebar!.querySelector<HTMLAnchorElement>('[data-nav-index]');
    firstLink?.focus();
  }

  function closeDrawer() {
    sidebar!.classList.remove('_sidebarOpen');
    backdrop!.classList.remove('_backdropVisible');
    toggle!.setAttribute('aria-expanded', 'false');
    toggle!.setAttribute('aria-label', 'Open navigation');
    document.removeEventListener('keydown', trapFocus);
    toggle!.focus();
  }

  function trapFocus(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closeDrawer();
      return;
    }
    if (e.key !== 'Tab') return;

    const focusable = sidebar!.querySelectorAll<HTMLElement>(
      'a[href], button, [tabindex]:not([tabindex="-1"])',
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  toggle.addEventListener('click', () => {
    const isOpen = toggle.getAttribute('aria-expanded') === 'true';
    isOpen ? closeDrawer() : openDrawer();
  });

  backdrop.addEventListener('click', closeDrawer);

  sidebar.querySelectorAll('[data-nav-index]').forEach((link) => {
    link.addEventListener('click', () => {
      if (window.innerWidth < 641) closeDrawer();
    });
  });
}

function scrollToHash() {
  const hash = window.location.hash;
  if (!hash) return;
  const el = document.querySelector(hash);
  if (el) {
    el.scrollIntoView({ behavior: 'smooth' });
    return;
  }
  const observer = new MutationObserver(() => {
    const target = document.querySelector(hash);
    if (target) {
      observer.disconnect();
      target.scrollIntoView({ behavior: 'smooth' });
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });
  setTimeout(() => observer.disconnect(), 5000);
}

initMobileDrawer();
document.addEventListener('astro:after-swap', () => {
  initMobileDrawer();
  scrollToHash();
});
