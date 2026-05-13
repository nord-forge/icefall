# IF-200: Complete framework deployment guides

**Phase:** 27 — Comprehensive Docs
**Priority:** High
**Estimate:** L

## Description

Write complete, tested deployment guides for every framework Icefall detects, plus popular frameworks that use Docker images. Each guide should take the user from `git init` to live production URL with best practices for that specific framework.

## Framework Guides to Complete

### Currently stubbed (need full content)
- [ ] `frameworks/vue.mdx` — Vue 3 + Vite
- [ ] `frameworks/nuxt.mdx` — Nuxt 3
- [ ] `frameworks/react.mdx` — React (Vite / CRA)
- [ ] `frameworks/nodejs.mdx` — Node.js (Express, Fastify, Hono)
- [ ] `frameworks/static.mdx` — Static sites (HTML/CSS/JS)
- [ ] `frameworks/dockerfile.mdx` — Custom Dockerfile

### New guides to create
- [ ] `frameworks/remix.mdx` — Remix
- [ ] `frameworks/sveltekit.mdx` — SvelteKit
- [ ] `frameworks/laravel.mdx` — Laravel (PHP)
- [ ] `frameworks/rails.mdx` — Ruby on Rails
- [ ] `frameworks/django.mdx` — Django (Python)
- [ ] `frameworks/flask.mdx` — Flask (Python)
- [ ] `frameworks/fastapi.mdx` — FastAPI (Python)
- [ ] `frameworks/go.mdx` — Go (any framework)
- [ ] `frameworks/rust.mdx` — Rust (Axum, Actix, Rocket)
- [ ] `frameworks/dotnet.mdx` — .NET / ASP.NET Core
- [ ] `frameworks/elixir.mdx` — Elixir / Phoenix
- [ ] `frameworks/docker-image.mdx` — Pre-built Docker images
- [ ] `frameworks/docker-compose.mdx` — Docker Compose stacks

### Each Guide Includes

- [ ] Prerequisites (runtime version, package manager)
- [ ] Project setup (if starting fresh)
- [ ] Required configuration (build command, start command, port)
- [ ] Environment variables specific to the framework
- [ ] Database connection (if applicable)
- [ ] Health check endpoint recommendation
- [ ] Static assets / CDN considerations
- [ ] Production optimizations (caching, compression, SSR vs SSG)
- [ ] Example `.env` file
- [ ] Common issues and fixes
- [ ] Example repo link (GitHub)
- [ ] Estimated build time and image size

## Standards

- [ ] Every guide tested end-to-end on a fresh Icefall instance
- [ ] Build commands verified with specific framework versions
- [ ] Screenshots of the deploy flow for each framework
- [ ] Code snippets are complete and copy-pasteable (not fragments)

## Dependencies

- IF-008 (Framework detection engine — verify detection accuracy per framework)
