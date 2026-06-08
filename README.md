# iam-rust

🌐 **English** | [Bahasa Indonesia](README.id.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-edition%202021-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![gRPC](https://img.shields.io/badge/gRPC-Tonic-244c5a)](https://github.com/hyperium/tonic)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

**Identity & Access Management** — Auth + User microservices with **granular
RBAC**, built in **Rust**. Sibling Go implementation: [`../iam-go`](../iam-go).

> Stack: **Rust · Axum** (REST gateway) · **Tonic/gRPC** (inter-service) ·
> **Tokio** · **PostgreSQL** · **sqlx** · **JWT** (access + refresh, revocable).

## Features

- 🔐 **Auth**: register, login, JWT access + refresh tokens with **rotation** and **revocation**.
- 👤 **Users**: profile CRUD + paginated search, via a dedicated service.
- 🛡️ **Granular RBAC**: roles → permissions; **dynamic** (role changes apply on the next request).
- 🧩 **Role management**: create/update/delete roles, grant/revoke permissions, assign/revoke roles.
- 🚪 **API Gateway**: single public entrypoint, REST→gRPC, per-route authorization.
- 📦 **Ready to run**: Docker Compose + Kubernetes manifests, auto migrations & seed, bootstrap admin.
- ✅ **Verified**: end-to-end smoke test + Postman/Bruno collections.

## Architecture

```
client ──REST──▶ Gateway (Axum) ──gRPC──▶ Auth Service ──▶ Postgres (auth_db)
                      │            └─gRPC──▶ User Service ──▶ Postgres (user_db)
                      └ validates JWT, resolves permissions, enforces RBAC per route
```

Full diagrams & flows: **[docs/en/architecture.md](docs/en/architecture.md)**.

## Quick start

```bash
make up        # build + run postgres + auth + user + gateway
make smoke     # end-to-end smoke test against http://localhost:8080
make down      # stop + remove volumes
```

A bootstrap admin (`admin@iam.local` / `admin12345`) is created on first boot.
Then:

```bash
# register, log in, and see your roles & permissions
curl -s localhost:8080/auth/register -H 'Content-Type: application/json' \
  -d '{"email":"alice@iam.local","password":"alicepass123"}'
TOKEN=$(curl -s localhost:8080/auth/login -H 'Content-Type: application/json' \
  -d '{"email":"alice@iam.local","password":"alicepass123"}' | jq -r .access_token)
curl -s localhost:8080/me -H "Authorization: Bearer $TOKEN"
```

## API

REST on `:8080`. Highlights: `/auth/{register,login,refresh,logout}`, `/me`,
`/users[/:id]`, `/roles`, `/permissions`, role/permission management. Full
reference with examples & error codes: **[docs/en/api-reference.md](docs/en/api-reference.md)**.

Try it with Postman or Bruno — see **[docs/en/api-collections.md](docs/en/api-collections.md)**.

## Project structure

```
proto/                  gRPC contracts
crates/proto/           tonic-build codegen        crates/common/   shared libs
crates/auth-service/    Auth gRPC service          crates/user-service/  User gRPC service
crates/gateway/         Axum REST gateway
deploy/                 compose · k8s              scripts/         smoke.sh
docs/                   en/ · id/ (bilingual)
```

## Documentation

Full bilingual docs in **[`docs/`](docs/en/README.md)**: Architecture · API
Reference · RBAC · Deployment · Development (with DB ERD) · API Collections.

## Development

```bash
make build     # cargo build --workspace
make test      # cargo test --workspace
make clippy    # cargo clippy
```

Requires the Rust toolchain + `protobuf-compiler` (for `tonic-build`). DB access
uses sqlx **runtime-checked** queries, so the Docker build is fully offline.
Details: **[docs/en/development.md](docs/en/development.md)**.

## Deployment

Docker Compose (local) and Kubernetes (kustomize) — see
**[docs/en/deployment.md](docs/en/deployment.md)**.

## Roadmap

- [ ] Rate limiting on auth endpoints
- [ ] Audit log for RBAC changes
- [ ] OpenAPI/Swagger spec generation
- [ ] Refresh-token reuse detection

## Contributing

Contributions welcome! See **[CONTRIBUTING.md](CONTRIBUTING.md)** and our
**[Code of Conduct](CODE_OF_CONDUCT.md)**.

## License

[MIT](LICENSE) © 2026 malvin
