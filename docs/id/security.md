# Security

🌐 [English](../en/security.md) | **Bahasa Indonesia**

Bagaimana IAM ini di-harden, ujung ke ujung — dari penanganan token hingga
cluster tempatnya berjalan. Model yang sama diimplementasikan di kedua stack,
[Go](https://github.com/malvinpratama/iam-go) maupun
[Rust](https://github.com/malvinpratama/iam-rust); deployment-nya ada di
[iam-gitops](https://github.com/malvinpratama/iam-gitops).

Prinsip pemandu sepanjang dokumen ini adalah **defense in depth dan fail-closed**:
setiap kontrol punya pengaman cadangan, dan ketika ada yang salah konfigurasi
sistem akan **menolak**, bukan mengizinkan.

---

## Autentikasi & sesi

- **Password hashing** — Argon2id (memory-hard) dengan salt per-user. Login
  menjalankan perbandingan constant-time terhadap hash dummy untuk user yang
  tidak dikenal, sehingga timing respons tidak membocorkan keberadaan akun.
- **JWT access token** — berumur pendek (15 menit), ditandatangani RS256; public
  key dipublikasikan sebagai JWKS. Setiap request divalidasi ulang di sisi server
  (bukan hanya dari signature): `jti` token dicek terhadap **denylist revocation**
  (Redis, dibagi lintas replica, dengan fallback Postgres), dan keanggotaan akun /
  tenant dicek ulang.
- **Refresh token** — berumur panjang, berotasi, dan bisa di-revoke. Rotasi
  mencakup **deteksi reuse**: memutar ulang token yang sudah dirotasi akan ditolak,
  tetapi dengan **grace window** pendek supaya refresh paralel yang ditembak
  browser saat token kedaluwarsa tidak meruntuhkan sesi (pola "reuse interval"
  ala Auth0/Okta).
- **Lockout akun** — kegagalan password (dan **MFA**) berulang mengunci akun
  selama cooldown, menahan brute force.
- **Ganti password self-service** — `POST /auth/password` memverifikasi password
  saat ini, menyetel yang baru, dan mencabut semua refresh token milik user.

## Autentikasi dua faktor

- **TOTP** opt-in (RFC 6238) dengan recovery code sekali-pakai; login menjadi
  challenge (password → token MFA berumur pendek → kode TOTP/recovery → token).
- **Secret TOTP terenkripsi at rest** dengan **AES-256-GCM** (kunci dari
  `TOTP_ENC_KEY`). Shared secret tidak bisa di-hash — ia harus bisa dipulihkan
  untuk menghitung rolling code — jadi dienkripsi dengan envelope berversi
  (`enc:v1:<nonce‖ciphertext>`), nonce acak per penulisan. Skema ini backward
  compatible: secret plaintext pra-enkripsi dibaca secara transparan dan
  di-upgrade jadi ciphertext pada enroll berikutnya, sehingga mengaktifkannya
  tidak butuh migrasi data.
- Recovery code di-**hash** (satu arah), tidak pernah dienkripsi.

## OIDC / OAuth2 provider

- Flow Authorization Code dengan **PKCE** (S256); authorization code bersifat
  **sekali-pakai** dan berumur pendek.
- `redirect_uri` dan `post_logout_redirect_uri` divalidasi terhadap allow-list
  terdaftar milik client (penjaga open-redirect).
- Client secret disimpan sebagai hash SHA-256; login/consent di browser dan
  langkah TOTP dibatasi rate-nya (tidak ada oracle password/OTP).
- ID token dan access token ditandatangani RS256 dan membawa binding tenant.

## Otorisasi & isolasi tenant

RBAC bersifat **role → permission**, dievaluasi per request dan **ter-scope ke
tenant token (dan project opsional)** — user yang sama bisa memegang permission
berbeda di organisasi berbeda. Isolasi ditegakkan di **dua lapis independen**:

1. **Lapis aplikasi** — setiap query ter-scope memfilter berdasarkan `tenant_id`
   aktif (dan project), dan gateway mengecek ulang permission yang dibutuhkan
   per route sementara setiap service mengecek ulang lagi (defense in depth —
   gateway tidak dipercaya sendirian).
2. **Row-Level Security PostgreSQL** — pengaman cadangan yang **fail-closed**.
   Read dan write ter-scope berjalan di dalam transaksi sebagai role non-superuser
   (`iam_rls`) dengan tenant diset via `SET LOCAL`; policy RLS (termasuk
   `WITH CHECK` pada write) memastikan query yang *lupa* `WHERE`-nya, atau `INSERT`
   lintas-tenant, tetap tidak bisa melewati batas.

Penjaga tenant tambahan: event audit distempel dan difilter berdasarkan tenant;
`GET /users/:id` lintas-tenant mengembalikan **404** (tidak membocorkan
keberadaan); API key terikat ke tenant/project dan dicek ulang saat dipakai;
**menangguhkan sebuah tenant** seketika membuat token para member-nya invalid.

> Runtime saat ini terhubung sebagai superuser yang *mem-bypass* RLS, jadi RLS
> hari ini hanya menggigit di dalam transaksi yang dibungkus `iam_rls`. Sebuah
> connection-role non-superuser (`iam_app`) telah disiapkan untuk cutover di masa
> depan yang membuat RLS ditegakkan di mana-mana — lihat
> [Roadmap](#kesenjangan-yang-diketahui--roadmap).

## Secret

- **Tidak ada secret plaintext di git.** Secret Kubernetes di-commit sebagai
  **Sealed Secrets** (dienkripsi dengan kunci controller in-cluster); hanya
  controller yang bisa men-dekripsinya jadi `Secret` sungguhan.
- **Penolakan placeholder** — proses produksi (`APP_ENV=production`) menolak boot
  jika `JWT_SECRET`, `BOOTSTRAP_ADMIN_PASSWORD`, `INTERNAL_SERVICE_TOKEN`, atau
  password database masih membawa marker demo/placeholder yang dikenal. Default
  yang tidak aman tidak bisa diam-diam masuk produksi.

## Service-to-service

- Gateway mengautentikasi ke service auth/user dengan **`INTERNAL_SERVICE_TOKEN`**
  bersama, ditegakkan **fail-closed**: token yang hilang/kosong menolak setiap
  panggilan internal (dibutuhkan `INTERNAL_AUTH_OPTIONAL=true` eksplisit untuk
  melonggarkannya saat dev lokal). Gateway tidak pernah meneruskan header
  identitas yang disuplai client — ia membangun ulang dari token yang sudah
  divalidasi, jadi tidak bisa dipalsukan.
- *Direncanakan:* mTLS antara gateway dan service (pembuatan cert sudah ada;
  wiring-nya masih menunggu).

## Network

- **NetworkPolicy default-deny**: setiap pod menolak semua ingress kecuali
  allow-list eksplisit — gateway hanya dari ingress controller, auth/user hanya
  dari gateway, tiap database hanya dari service-nya sendiri, NATS/Redis hanya
  dari pemanggilnya. Pod yang dikompromikan tidak bisa bergeser lateral.

## Hardening container & pod

Setiap container aplikasi berjalan:

- **non-root** dengan UID tetap dan `runAsNonRoot`,
- **root filesystem read-only** (satu `emptyDir` kecil di `/tmp` untuk scratch),
- **semua capability Linux di-drop**, tanpa privilege escalation, `seccompProfile:
  RuntimeDefault`,
- **request + limit CPU/memori**, dan
- **token service-account di-unmount** (`automountServiceAccountToken: false`).

## Supply chain & deployment

- **Pure GitOps** — state cluster ada di git ([iam-gitops](https://github.com/malvinpratama/iam-gitops));
  ArgoCD merekonsiliasinya. Tidak ada perubahan imperatif yang bertahan.
- **Image pinning immutable** — deployment mereferensikan image tag `sha-<commit>`,
  tidak pernah `:latest`. Setiap rollout deterministik, auditable, dan reversible
  (rollback dengan mengarahkan tag ke commit sebelumnya).

## Observability

Distributed tracing (OpenTelemetry → Jaeger), metrik Prometheus, dan
`X-Request-Id` yang dikorelasikan lintas log dan trace — sehingga kegagalan auth
atau request yang ditolak bisa dilacak ujung ke ujung. Material sensitif (mis.
body token email-verification / password-reset) disembunyikan dari log di luar
development.

## Kesenjangan yang diketahui & roadmap

Jujur soal apa yang belum selesai:

- **mTLS** gateway↔service — ditunda (gRPC plaintext di network internal saat ini,
  dimitigasi NetworkPolicy + token internal).
- **Egress NetworkPolicy** — sejauh ini baru ingress yang default-deny; egress
  masih terbuka (DNS, DB, NATS, Redis, OIDC discovery).
- **Cutover DB least-privilege** — role non-superuser `iam_app` sudah disiapkan
  tetapi koneksi masih berjalan sebagai superuser yang mem-bypass RLS; cutover
  butuh sisa hot path dibungkus dulu.
- **Tabel OAuth di bawah RLS** — `oauth_authorization_codes` / `oauth_consents`
  belum ter-scope tenant di lapis database.

---

*Menemukan sesuatu? Isu keamanan diterima lewat laporan privat — lihat
[CONTRIBUTING.md](../../CONTRIBUTING.md).*
