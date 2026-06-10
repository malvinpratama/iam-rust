# Changelog

All notable changes to **iam-rust** are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-06-11

### Added (v0.9 — M4: enterprise auth)
- **2FA / TOTP** — opt-in self-service: `POST /auth/2fa/enroll` (secret + otpauth
  URI + one-time recovery codes), `/activate`, `/disable`. Login becomes a
  challenge: a password login returns `mfa_required` + a short-lived `mfa_token`,
  then `POST /auth/login/totp` with a TOTP or recovery code issues the token pair.
- **Scoped API keys** (`iamk_…`) — `POST`/`GET`/`DELETE /api-keys`. Requested
  scopes must be a subset of the creator's permissions; the effective scope is
  `requested ∩ the owner's current permissions`. Stored as a SHA-256 hash, with
  optional expiry and revoke. The gateway authenticates `Authorization: Bearer
  iamk_…` via ValidateApiKey.
- **Soft-delete + restore** — `deleted_at` on identity and profile.
  `DELETE /users/:id` soft-deletes (login blocked, profile hidden, sessions
  revoked); `POST /users/:id/restore` reverses it; `DELETE /users/:id?hard=true`
  removes permanently. Driven by `UserDeleted{hard}` / `UserRestored` events.

### Changed
- Access-token validation now rejects soft-deleted accounts and MFA-purpose tokens.

## [0.8.0] - 2026-06-11

### Added (v0.8 — horizontal scale)
- **Redis-backed rate limiter** — the per-IP auth limiter now counts in Redis
  (atomic `INCR`+`EXPIRE`), so the cap is enforced **globally across gateway
  replicas** instead of per-pod. Falls back to in-memory when `REDIS_URL` is
  unset (single-instance/dev). Fail-open on Redis error.
- **Multi-replica gateway** — the demo cluster runs the gateway at **2 replicas**
  behind a shared, ephemeral **Redis**; both stacks gain a `redis` Deployment and
  `REDIS_URL` config.

### Changed
- `BENCHMARKS.md` gains a **Horizontal scale** section proving the shared cap
  (5 global vs 5×2 per-pod: 8 requests → 5 passed, 3× `429`).

## [0.7.0] - 2026-06-10

### Added (v0.7 — OIDC / OAuth2 provider)
- **OpenID Connect provider** at the gateway: discovery
  (`/.well-known/openid-configuration`), **JWKS** (`/.well-known/jwks.json`),
  **Authorization Code + PKCE** flow (`/authorize` with a login form + consent
  screen + signed session cookie), **token endpoint** (`/token` —
  `authorization_code` & `refresh_token`, `client_secret_basic`/`_post`/PKCE),
  **UserInfo** (`/userinfo`), and **client registration** (`/oauth/clients`).
- **ID tokens** (RS256) with `iss`/`sub`/`aud`/`email`/`nonce`, verifiable by
  relying parties via JWKS.
- A demo confidential **console client** is seeded on first boot
  (`OIDC_CONSOLE_*` env).
- Swagger UI documents all OIDC endpoints.

### Changed
- **Token signing migrated HS256 → RS256** (asymmetric, rotatable keys with
  `kid`); token validation stays centralized in the auth service.

### Notes
- New deploy env: `OIDC_ISSUER`, `SESSION_SECRET`, `OIDC_CONSOLE_*`.

## [0.6.5] - 2026-06-10

### Changed
- **ROADMAP refreshed** to match reality: v0.5 (Observability) and v0.6 (Show it
  off — Swagger, live k3s demo, k6 benchmark, configurable rate limit) marked
  shipped; next milestones planned — **v0.7 OIDC/OAuth2 provider**, **v0.8
  horizontal scale + Redis**.

## [0.6.4] - 2026-06-10

### Changed
- **BENCHMARKS.md: added rate-limiter-OFF results.** With `AUTH_RATE_LIMIT=0`
  the mixed load runs at **0% errors** (final proof the earlier ~9% was the auth
  rate limiter, not argon2), and the Go/Rust ranking flips: **Go** leads the
  I/O-bound read path, **Rust** leads the CPU-bound argon2 login path.

## [0.6.3] - 2026-06-10

### Added
- **Configurable auth rate limit**: the gateway's per-IP `/auth/*` limiter now
  reads `AUTH_RATE_LIMIT` (requests, default 60) and `AUTH_RATE_WINDOW_SECONDS`
  (window seconds, default 60); `AUTH_RATE_LIMIT=0` disables it. Was hard-coded.

## [0.6.2] - 2026-06-10

### Fixed
- **BENCHMARKS.md analysis corrected**: the ~9% error under mixed load is the
  gateway's per-IP **auth rate limiter** (HTTP 429, 60 req/min on `/auth/*`) —
  brute-force protection working as designed — **not** argon2 saturation (the
  v0.6.1 note was wrong). Added a login-only status-code breakdown (60 logins
  allowed, then 429) confirming it. Note: Rust emitted ~1% 5xx under the extreme
  reject load where Go emitted none.

### Changed
- Roadmap: marked **rate limiting on auth endpoints** done — it already ships.

## [0.6.1] - 2026-06-10

### Changed
- **Live demo is up**: README points at the running stacks on k3s via ArgoCD —
  `https://iam-rust.digitalglobalgrowth.com/docs/` and
  `https://iam-go.digitalglobalgrowth.com/docs/` (interactive Swagger UI).
- **BENCHMARKS.md filled with real numbers**: Go-vs-Rust head-to-head on the
  same single-node cluster (off-node k6, no Cloudflare). Read-path 631 vs 649
  req/s, p95 21.6 vs 17.5 ms, 0% errors — within ~3%; Rust has the tighter
  worst-case tail under mixed load. argon2 + infra sizing dominate, not the
  language. Added an in-cluster k6 Job (`iam-gitops/bench`).

## [0.6.0] - 2026-06-10

### Added (v0.6 — Show it off)
- **Interactive API**: the gateway serves **Swagger UI** at `/docs` and the
  OpenAPI 3 spec at `/openapi.yaml` (vendored, no CDN). Authorize with a Bearer
  token and try every endpoint live — no separate frontend needed.
- **Live demo via GitOps**: deployable to **k3s with ArgoCD** — see the
  `iam-gitops` repo (ArgoCD Applications + kustomize overlays + Traefik ingress),
  both stacks side by side.
- **Go-vs-Rust benchmark harness**: `bench/load.js` (k6) + `BENCHMARKS.md`
  (methodology + comparison table) for a fair head-to-head on identical infra.

## [0.5.1] - 2026-06-09

### Changed
- **Linked cross-service tracing**: upgraded the services to **tonic 0.14** and
  added the tonic OpenTelemetry middleware — the gateway injects trace context
  into outgoing gRPC (client layer) and auth/user continue the trace (server
  layer). A request now appears as one linked trace (gateway → auth/user) in
  Jaeger, matching the Go stack. Resolves the v0.5.0 follow-up note.

## [0.5.0] - 2026-06-09

### Added (v0.5 — Observability)
- **Distributed tracing** (OpenTelemetry → Jaeger): the gateway traces every
  request and key auth/user handlers are instrumented; spans export via OTLP.
  Optional via `OTEL_EXPORTER_OTLP_ENDPOINT`. (Cross-service span linking is
  richer on the Go stack; the Rust tonic tracing layer is a follow-up.)
- **Prometheus metrics**: the gateway exposes `/metrics` (axum-prometheus),
  scraped by a bundled Prometheus.
- **Grafana** with a provisioned Prometheus datasource and an "IAM Overview"
  dashboard (request rate, p95 latency, status, in-flight).
- **Correlation IDs**: `X-Request-Id` accepted/generated and echoed per request.
- Compose adds `jaeger` (UI :16686), `prometheus` (:9090), `grafana` (:3000).

## [0.4.0] - 2026-06-09

### Changed (v0.4 — True microservices)
- **Separate repositories per service**: this repo is now the platform/umbrella;
  each service lives in its own repo
  ([iam-rust-gateway](https://github.com/malvinpratama/iam-rust-gateway),
  [iam-rust-auth](https://github.com/malvinpratama/iam-rust-auth),
  [iam-rust-user](https://github.com/malvinpratama/iam-rust-user)) with shared
  crate repos ([iam-rust-proto](https://github.com/malvinpratama/iam-rust-proto),
  [iam-rust-common](https://github.com/malvinpratama/iam-rust-common)), pinned by
  git tag. Each is built, versioned and deployed independently.
- **One database instance per service** (`postgres-auth`, `postgres-user`).
- **Event-driven cross-service flow**: register/delete no longer orchestrate
  synchronously at the gateway. Auth writes a **transactional outbox** in the
  same DB transaction; a relay (tokio task) publishes to **NATS JetStream**; the
  user service consumes idempotently to create/drop the profile. `GET /users/me`
  lazy-heals as the eventual-consistency safety net. The broker is optional
  (`NATS_URL`).
- **CI/CD per repo** (GitHub Actions): build + test, and service images
  published to GHCR. Umbrella compose pulls the images.

### Future work
- Compensation saga (`iam.user.registration_failed`) for permanently-failed
  profile creation — not needed today thanks to idempotent upsert + lazy heal.

### Added (v0.2 — Security+)
- **Account recovery**: email verification (`/auth/verify-email/request`,
  `/auth/verify-email`) and password reset (`/auth/password-reset/request`,
  `/auth/password-reset`). In non-production the token is returned as `dev_token`;
  otherwise it is emailed (tracing-based sender by default).
- **Audit log** of sensitive actions (`audit_events`), readable at `GET /audit`
  (`audit:read`, admin).
- **Account lockout**: lock after `LOGIN_MAX_FAILURES` failed logins for
  `LOGIN_LOCKOUT_SECONDS` (configurable; `0` disables).
- **Refresh-token reuse detection**: presenting an already-revoked refresh token
  revokes the user's whole token family.
- Optional **email-verification gate** on login (`REQUIRE_EMAIL_VERIFICATION`).
- Opt-in **TLS** cert generator (`scripts/gen-certs.sh`) + production hardening
  and secrets-management docs (Vault / Sealed Secrets / External Secrets).
- All toggles default to non-breaking; the existing smoke flow is unchanged.

### Added
- Auth service (gRPC): register, login, refresh (with rotation), logout
  (revocation), `ValidateToken`.
- User service (gRPC): profile create/get/update/delete, paginated list/search.
- API Gateway (Gin, REST→gRPC) with JWT auth middleware and per-route RBAC.
- Granular RBAC: roles + permissions, seeded `admin`/`user`.
- `GET /me` — caller's own roles & permissions.
- `GET /permissions` — list all permissions (`role:read`).
- Role management: `POST/PATCH/DELETE /roles`, grant/revoke permission to a role
  (`role:write`); built-in roles protected from deletion.
- Assign/revoke role to a user (`role:assign`).
- JWT access + refresh tokens; refresh tokens hashed & revocable in the DB.
- Embedded migrations + RBAC seed run at startup; bootstrap admin on first boot.
- Docker Compose + Kubernetes (kustomize) manifests; health checks.
- End-to-end smoke test (`scripts/smoke.sh`).
- Postman & Bruno API collections.
- Bilingual documentation (English + Indonesian) under `docs/`.

### Security
- Fixed broken object-level authorization: editing another user's profile now
  requires `user:write` (admin); `profile:write` covers only your own profile.
- `DELETE /users/:id` now deletes the identity (credentials, roles, refresh
  tokens) as well as the profile — a deleted user can no longer log in.
- Defense in depth: internal services re-check permissions and require a shared
  `INTERNAL_SERVICE_TOKEN` from the gateway; Kubernetes `NetworkPolicy` restricts
  service-to-service traffic.
- Access tokens are now revocable: logout denylists the token by `jti`.
- Constant-time login (dummy hash on unknown users) to reduce user enumeration.
- Per-IP rate limiting on `/auth/*`; request body-size limit.
- Startup security guard (rejects default JWT secret / admin password / missing
  internal token when `APP_ENV=production`); Postgres bound to localhost.
- Passwords hashed with argon2; gRPC reflection not exposed in production.

[Unreleased]: https://github.com/malvin/iam-rust
