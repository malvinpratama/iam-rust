# Arsitektur — iam-rust

🌐 [English](../en/architecture.md) | **Bahasa Indonesia** · [↑ Indeks dokumentasi](README.md)

## Ikhtisar

`iam-rust` adalah sistem Identity & Access Management yang dibagi menjadi tiga layanan:

- **Auth Service** (gRPC) — sumber kebenaran untuk identitas, kredensial, token
  JWT, dan seluruh RBAC (role, permission, penetapan).
- **User Service** (gRPC) — profil user, dikunci dengan `user_id` kanonik yang
  dibuat oleh Auth.
- **API Gateway** (Axum, REST) — satu-satunya entrypoint publik. Memvalidasi JWT,
  menyelesaikan permission pemanggil, menegakkan RBAC per route, dan
  menerjemahkan REST → gRPC.

PostgreSQL adalah datastore-nya (satu database logis per layanan: `auth_db`,
`user_db`).

## Diagram komponen

```mermaid
flowchart LR
    C[Client<br/>curl / Postman / Bruno] -->|REST/JSON| GW[API Gateway<br/>Axum]
    GW -->|gRPC| AUTH[Auth Service]
    GW -->|gRPC| USER[User Service]
    AUTH --> ADB[(Postgres<br/>auth_db)]
    USER --> UDB[(Postgres<br/>user_db)]
    GW -.->|ValidateToken| AUTH
```

## Tanggung jawab

| Layanan | Memiliki | Operasi utama |
|---|---|---|
| Auth | `users`, `refresh_tokens`, `roles`, `permissions`, `role_permissions`, `user_roles` | Register, Login, Refresh, Logout, ValidateToken, manajemen RBAC |
| User | `profiles` | CreateProfile, GetProfile, UpdateProfile, DeleteProfile, ListProfiles |
| Gateway | tidak ada (stateless) | AuthN (JWT), AuthZ (pengecekan permission), REST↔gRPC, orkestrasi registrasi |

Layanan internal mempercayai identitas yang ditaruh gateway di metadata gRPC
(`x-user-id`, `x-user-email`, `x-user-roles`, `x-user-permissions`) karena hanya
gateway yang dapat dijangkau dari luar; layanan-layanan tersebut berada di
jaringan internal.

## Alur: login + request terautentikasi

```mermaid
sequenceDiagram
    participant C as Client
    participant GW as Gateway
    participant A as Auth
    participant U as User
    C->>GW: POST /auth/login {email, password}
    GW->>A: Login()
    A->>A: verify password (argon2), issue JWT + refresh
    A-->>GW: access_token, refresh_token
    GW-->>C: 200 token pair
    C->>GW: GET /users/:id (Bearer access_token)
    GW->>A: ValidateToken(access_token)
    A-->>GW: user_id, roles, permissions
    GW->>GW: require "user:read"?
    alt has permission
        GW->>U: GetProfile() + identity metadata
        U-->>GW: profile
        GW-->>C: 200 profile
    else missing permission
        GW-->>C: 403 Forbidden
    end
```

## Alur: registrasi (orkestrasi gateway)

```mermaid
sequenceDiagram
    participant C as Client
    participant GW as Gateway
    participant A as Auth
    participant U as User
    C->>GW: POST /auth/register {email, password}
    GW->>A: Register() → mint user_id, assign role "user"
    A-->>GW: user_id
    GW->>U: CreateProfile(user_id, display_name)
    U-->>GW: profile
    GW-->>C: 201 {user_id, email}
```

## Token

- **Access token**: JWT berumur pendek (HS256), membawa `sub` (user_id) + `email`.
  Permission TIDAK ditanamkan ke dalam token — keduanya diselesaikan secara segar
  dari DB pada setiap pemanggilan `ValidateToken`, sehingga perubahan role berlaku
  segera (RBAC dinamis).
- **Refresh token**: string acak opaque berumur panjang. Hanya hash SHA-256-nya
  yang disimpan; token dapat dicabut (logout) dan dirotasi pada setiap refresh.

Lihat juga: [model RBAC](rbac.md) · [referensi API](api-reference.md) ·
[Pengembangan](development.md) untuk ERD database.
