# IF-181: Built-in API playground

**Phase:** 25 — Icefall+
**Priority:** Low
**Estimate:** M

## Description

Embed an interactive API playground in the dashboard that lets users explore and test the Icefall API without external tools. Pre-authenticated with the current session, auto-populated with the OpenAPI spec (IF-036). Like a built-in Postman/Hoppscotch scoped to the Icefall API.

## Acceptance Criteria

- [ ] New "API" page accessible from the sidebar (or Settings sub-page)
- [ ] Endpoint browser: grouped by resource (apps, databases, deploys, servers, etc.)
- [ ] For each endpoint: method, path, description, parameters, request body schema
- [ ] "Try it" panel: fill in parameters, send the request, see the response
- [ ] Auto-authenticated: uses the current session cookie (no manual token entry needed)
- [ ] Response display: formatted JSON with syntax highlighting, status code, timing
- [ ] Request history: last 20 requests with replay button
- [ ] "Copy as curl" button for each request
- [ ] Generated from the OpenAPI spec at `/api/v1/openapi.json` — always up to date
- [ ] Code generation: show example code in `curl`, `JavaScript (fetch)`, and `Python (requests)` for each endpoint

## Technical Notes

- Consider embedding Scalar (https://github.com/scalar/scalar) or Swagger UI as a Preact island
- Scalar is lightweight, modern, and supports OpenAPI 3.1 — much nicer than Swagger UI
- The OpenAPI spec (IF-036) is the single source of truth — the playground reads it at runtime
- Auth: inject the session cookie automatically, or allow entering an API token for testing token auth

## Dependencies

- IF-036 (OpenAPI specification)
