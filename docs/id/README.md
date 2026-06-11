# Dokumentasi iam-rust

🌐 [English](../en/README.md) | **Bahasa Indonesia** · [← README Proyek](../../README.md)

Identity & Access Management — **Microservice Auth + User dengan RBAC granular**,
dibangun dengan **Rust** (Axum · Tokio · Tonic · sqlx · PostgreSQL · JWT).

## Daftar Isi

| Dokumen | Isinya |
|---|---|
| [Arsitektur](architecture.md) | Layanan, diagram komponen & sekuens, model token |
| [Referensi API](api-reference.md) | Setiap endpoint REST (request/response/error) + kontrak gRPC |
| [Model RBAC](rbac.md) | Role, permission, seed, RBAC dinamis, manajemen role |
| [Multi-tenant](multi-tenant.md) | Tenant/project/membership, token terikat tenant, RBAC ter-scope, RLS, OIDC client→tenant |
| [Deployment & Ops](deployment.md) | Docker Compose, Kubernetes, env var, migrasi, pemecahan masalah |
| [Pengembangan](development.md) | Toolchain, codegen, struktur proyek, test, **ERD DB** |
| [Koleksi API](api-collections.md) | Penggunaan Postman & Bruno (dua koleksi native) |

## Mulai cepat

```bash
make up        # build + run the full stack (postgres + auth + user + gateway)
make smoke     # end-to-end smoke test against http://localhost:8080
make down      # stop + remove volumes
```

Admin bootstrap (`admin@iam.local` / `admin12345`) dibuat saat boot pertama.
Daftarkan seorang user, login, dan panggil `GET /me` untuk melihat role & permission Anda.

## Sekilas

```
client ──REST──▶ Gateway (Axum) ──gRPC──▶ Auth Service ──▶ Postgres (auth_db)
                     │            └─gRPC──▶ User Service ──▶ Postgres (user_db)
                     └ validates JWT, resolves permissions, enforces RBAC per route
```

Implementasi paralel dalam Go berada di iam-go (https://github.com/malvinpratama/iam-go).
