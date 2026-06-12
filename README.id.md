# iam-rust

🌐 [English](README.md) | **Bahasa Indonesia**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-edition%202021-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![gRPC](https://img.shields.io/badge/gRPC-Tonic-244c5a)](https://github.com/hyperium/tonic)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

**Identity & Access Management** — microservice Auth + User dengan **RBAC
granular**, dibangun dengan **Rust**. Ini repo **platform/umbrella**: meng-orkestrasi
service yang di-deploy independen, serta memuat deployment, dokumentasi, dan
koleksi API. Implementasi Go pendamping:
[iam-go](https://github.com/malvinpratama/iam-go).

> Stack: **Rust · Axum** (REST gateway) · **Tonic/gRPC** (antar-service) ·
> **NATS JetStream** (event async) · **Tokio** · **PostgreSQL** (satu DB per
> service) · **sqlx** · **JWT** (access + refresh, bisa di-revoke).

## Repositori

Tiap service adalah repo tersendiri — di-build, di-versioning, dan di-deploy
independen; kode bersama ada di repo crate khusus.

| Repo | Peran |
|---|---|
| [iam-rust-gateway](https://github.com/malvinpratama/iam-rust-gateway) | API gateway REST→gRPC, otorisasi per-route |
| [iam-rust-auth](https://github.com/malvinpratama/iam-rust-auth) | Service Auth + RBAC (pemilik `auth_db`, penerbit event) |
| [iam-rust-user](https://github.com/malvinpratama/iam-rust-user) | Service profil (pemilik `user_db`, konsumen event) |
| [iam-rust-proto](https://github.com/malvinpratama/iam-rust-proto) | `.proto` bersama + crate kontrak tonic-build |
| [iam-rust-common](https://github.com/malvinpratama/iam-rust-common) | Crate bersama (config, jwt, argon2, NATS, …) |
| **iam-rust** (repo ini) | Platform: compose · k8s · docs · koleksi · smoke |

## Fitur

- 🔐 **Auth**: register, login, JWT access + refresh token dengan **rotasi** (deteksi reuse berikut grace window) dan **revocation**; **ganti password** self-service; lockout akun saat brute force.
- 🔑 **2FA / TOTP**: 2FA opt-in via aplikasi authenticator dengan recovery code sekali-pakai; shared secret **terenkripsi at rest** (AES-256-GCM).
- 🪪 **OIDC / OAuth2 provider**: Authorization Code + **PKCE**, discovery document, JWKS, `/userinfo`, registrasi client dinamis, RP-initiated logout — login ke console (atau RP mana pun) lewat flow milik IAM sendiri.
- 🎫 **API key ter-scope**: key `iamk_…` (di-hash SHA-256), scope ⊆ permission pemilik saat itu.
- 🛡️ **RBAC granular**: role → permission; **dinamis** (perubahan role berlaku di request berikutnya); **ter-scope per tenant/project**; manajemen role/permission penuh.
- 🏢 **Multi-tenant** (v0.10): tenant/project/membership, token terikat tenant + switcher, OIDC client→tenant, isolasi app-layer **+ Postgres RLS** — lihat **[docs/id/multi-tenant.md](docs/id/multi-tenant.md)**.
- 🔒 **Security-hardened** (v0.11): secret 2FA terenkripsi, auth internal fail-closed, write yang ditegakkan RLS, edge hardening di gateway, Sealed Secrets, NetworkPolicy default-deny, container non-root + read-only, image pin yang immutable — lihat **[docs/id/security.md](docs/id/security.md)**.
- 👤 **Users**: CRUD profil + pencarian berpaginasi, lewat service tersendiri; audit log.
- 🚪 **API Gateway**: satu pintu masuk publik, REST→gRPC, otorisasi per-route.
- 📦 **Siap jalan**: Docker Compose + manifest Kubernetes, migrasi & seed otomatis, bootstrap admin.
- ✅ **Terverifikasi**: smoke test end-to-end + **integration test Postgres** (Testcontainers) + koleksi Postman/Bruno.

## Arsitektur

```
client ──REST──▶ Gateway (Axum) ──gRPC──▶ Auth Service ──▶ Postgres (auth_db)
                      │            └─gRPC──▶ User Service ──▶ Postgres (user_db)
                      │                          ▲
                      │   register / delete      │ konsumsi
                      └ validasi JWT, RBAC       │
                                                 │
        Auth ──outbox──▶ NATS JetStream ──iam.user.*──┘   (async, eventually consistent)
```

Auth dan User tidak saling memanggil: efek lintas-service (buat profil saat
register, hapus profil saat delete) mengalir lewat **outbox transaksional →
NATS JetStream → konsumen idempoten**. Diagram & alur lengkap:
**[docs/id/architecture.md](docs/id/architecture.md)**.

## Mulai cepat

```bash
make up                 # tarik image service dari GHCR + jalankan stack
make up IMAGE_TAG=dev   # atau pakai image hasil build lokal
make smoke              # smoke test end-to-end ke http://localhost:8080
make down               # hentikan + hapus volume
```

Bootstrap admin (`admin@iam.local` / `admin12345`) dibuat saat pertama boot. Lalu:

```bash
# register, login, dan lihat role & permission milikmu
curl -s localhost:8080/auth/register -H 'Content-Type: application/json' \
  -d '{"email":"alice@iam.local","password":"alicepass123"}'
TOKEN=$(curl -s localhost:8080/auth/login -H 'Content-Type: application/json' \
  -d '{"email":"alice@iam.local","password":"alicepass123"}' | jq -r .access_token)
curl -s localhost:8080/me -H "Authorization: Bearer $TOKEN"
```

## Demo live & benchmark

Kedua stack berjalan live di **k3s via ArgoCD (GitOps)**, berdampingan, di belakang Cloudflare:

- **Go** — Swagger interaktif: **https://iam-go.digitalglobalgrowth.com/docs/**
- **Rust** — Swagger interaktif: **https://iam-rust.digitalglobalgrowth.com/docs/**

Login dengan **akun demo read-only** `demo@iam.local` / `demo1234`
(Authorize → Bearer), lalu coba endpoint mana pun — akun ini bisa membaca semua
tetapi tidak bisa mengubah apa pun. Admin console berjalan di atas kedua backend
di **https://console.digitalglobalgrowth.com** (kredensial demo yang sama),
dengan switch live antara backend Go dan Rust. Beban k6 yang sama dijalankan ke
keduanya untuk perbandingan Go-vs-Rust — lihat **[BENCHMARKS.md](BENCHMARKS.md)**
(`bench/load.js`).

## API

REST di `:8080`. Sorotan: `/auth/{register,login,refresh,logout}`, `/me`,
`/users[/:id]`, `/roles`, `/permissions`, manajemen role/permission. Referensi
lengkap dengan contoh & kode error:
**[docs/id/api-reference.md](docs/id/api-reference.md)**.

Coba lewat Postman atau Bruno — lihat
**[docs/id/api-collections.md](docs/id/api-collections.md)**.

## Struktur project

Repo umbrella ini hanya memuat lapisan platform; kode service ada di
[repo per-service](#repositori).

```
deploy/       docker-compose · k8s · .env.example
docs/         en/ · id/ (dwibahasa)
scripts/      smoke.sh
*.json        koleksi Postman + environment
```

## Dokumentasi

Dokumentasi lengkap dwibahasa di **[`docs/`](docs/id/README.md)**: Arsitektur ·
API Reference · RBAC · **[Security](docs/id/security.md)** · Multi-tenant ·
Deployment · Development (dengan ERD DB) · API Collections.

## Pengembangan

Tiap service dikembangkan di repo masing-masing (`make build` / `make test` /
`make docker` di sana). Crate `proto` & `common` di-tag; service mem-pin versi
pasti lewat git dependency. Untuk kerja lintas-repo, checkout repo berdampingan
dan override git dep dengan `[patch]` lokal (jangan di-commit). Butuh toolchain
Rust + `protobuf-compiler`. Akses DB memakai query sqlx **runtime-checked**, jadi
build Docker sepenuhnya offline. Detail:
**[docs/id/development.md](docs/id/development.md)**.

## Deployment

Docker Compose (lokal) dan Kubernetes (kustomize) — lihat
**[docs/id/deployment.md](docs/id/deployment.md)**.

## Roadmap

- [x] Rate limiting di endpoint auth (berbasis Redis, dibagi lintas replica)
- [x] OIDC / OAuth2 provider — Authorization Code + PKCE (v0.7)
- [x] 2FA / TOTP, API key ter-scope, soft-delete (v0.9)
- [x] Multi-tenant + multi-project dengan Postgres RLS (v0.10)
- [x] Audit log + deteksi reuse refresh-token + OpenAPI/Swagger UI
- [x] Security hardening — 2FA terenkripsi, Sealed Secrets, NetworkPolicy, image pinning (v0.11)
- [ ] mTLS antara gateway dan service
- [ ] Egress NetworkPolicy + cutover ke connection-role DB least-privilege

## Berkontribusi

Kontribusi sangat diterima! Lihat **[CONTRIBUTING.md](CONTRIBUTING.md)** dan
**[Code of Conduct](CODE_OF_CONDUCT.md)**.

## Lisensi

[MIT](LICENSE) © 2026 malvin
