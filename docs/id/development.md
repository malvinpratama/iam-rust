# Panduan Pengembangan — iam-rust

🌐 [English](../en/development.md) | **Bahasa Indonesia** · [↑ Indeks dokumentasi](README.md)

## Toolchain

- Rust (stable, edition 2021) + Cargo
- `protobuf-compiler` (untuk tonic-build)
- Docker + Docker Compose

## Perintah umum

```bash
make build     # cargo build --workspace
make test      # cargo test --workspace
make clippy    # cargo clippy --workspace
make up        # docker compose up --build -d
make smoke     # scripts/smoke.sh http://localhost:8080
make down      # docker compose down -v
```

## Pembuatan kode (code generation)

Tidak ada langkah pembuatan proto/sqlc terpisah:

- **gRPC**: proto dibuat oleh `tonic-build` di `crates/proto/build.rs` saat waktu
  kompilasi, dari kontrak kanonik di `proto/**`.
- **SQL**: akses DB menggunakan query sqlx yang **diperiksa saat runtime**
  (`query_as` / `query_scalar`), sehingga tidak ada codegen dan tidak perlu DB
  yang hidup untuk mengompilasi (build Docker sepenuhnya offline).

## Struktur proyek

```
proto/                       canonical gRPC contracts
crates/proto/                tonic-build codegen (build.rs)
crates/common/               shared: config, jwt, password (argon2), telemetry
crates/auth-service/         Auth gRPC service (grpc.rs, repo.rs, migrations)
crates/user-service/         User gRPC service
crates/gateway/              Axum REST gateway (router.rs, middleware.rs, clients.rs, error.rs)
deploy/                      docker-compose, .env, postgres-init, k8s
scripts/smoke.sh             end-to-end test
```

Jalur request: `crates/gateway/src/router.rs` → `middleware.rs` (AuthN + AuthZ
melalui ekstraktor Identity) → klien tonic (`clients.rs`) → layanan `grpc.rs` →
`repo.rs` (sqlx) → Postgres.

## Pengujian

`make test` menjalankan unit test (mis. JWT sign/verify/expiry di
`crates/common`). Perilaku end-to-end (alur auth, rotasi refresh, pencabutan,
RBAC dinamis) dicakup oleh `scripts/smoke.sh` terhadap stack yang sedang berjalan.

## Skema database (ERD)

```mermaid
erDiagram
    users ||--o{ refresh_tokens : has
    users ||--o{ user_roles : has
    roles ||--o{ user_roles : in
    roles ||--o{ role_permissions : grants
    permissions ||--o{ role_permissions : in

    users {
        uuid id PK
        text email UK
        text password_hash
        text status
        timestamptz created_at
        timestamptz updated_at
    }
    refresh_tokens {
        uuid id PK
        uuid user_id FK
        text token_hash UK
        timestamptz expires_at
        timestamptz revoked_at
        timestamptz created_at
    }
    roles {
        bigserial id PK
        text name UK
        text description
    }
    permissions {
        bigserial id PK
        text name UK
        text description
    }
    role_permissions {
        bigint role_id FK
        bigint permission_id FK
    }
    user_roles {
        uuid user_id FK
        bigint role_id FK
    }
    profiles {
        uuid user_id PK
        text display_name
        text bio
        text avatar_url
        text phone
        timestamptz created_at
        timestamptz updated_at
    }
```

`users`, `refresh_tokens`, `roles`, `permissions`, `role_permissions`,
`user_roles` berada di **auth_db**; `profiles` berada di **user_db** (dikunci
dengan `user_id` yang dibuat oleh Auth — tanpa FK lintas database).

## Konvensi

- Conventional Commits (lihat [CONTRIBUTING](../../CONTRIBUTING.md)).
- Jalankan `cargo fmt` dan `cargo clippy` sebelum commit.
- Jaga handler tetap tipis; letakkan SQL di `repo.rs`.
- Petakan error domain ke kode tonic `Status`; gateway memetakannya ke status
  HTTP (`crates/gateway/src/error.rs`).
- Perbarui dokumen di **kedua** `docs/en` dan `docs/id` ketika perilaku berubah.
