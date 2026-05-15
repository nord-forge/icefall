fetch('/api/v1/users/me', { credentials: 'same-origin' })
  .then((r) => {
    if (r.status === 401 || r.status === 400) {
      fetch('/api/v1/onboarding/status', { credentials: 'same-origin' })
        .then((or) => or.json())
        .then((ob) => {
          window.location.href = ob.is_complete ? '/login' : '/onboarding';
        })
        .catch(() => {
          window.location.href = '/login';
        });
    }
  })
  .catch(() => {});
