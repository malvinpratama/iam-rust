# Contributing to iam-rust

🌐 **English** | [Bahasa Indonesia](#berkontribusi-bahasa-indonesia)

Thanks for your interest in contributing! This document explains how to set up
the project, the conventions we follow, and how to submit changes.

## Prerequisites

- Rust (stable, edition 2021) + Cargo
- `protobuf-compiler` (`protoc`) — required by `tonic-build` for local builds
- Docker + Docker Compose (for the full stack; the Docker build needs no local
  Rust toolchain)

## Local setup

```bash
make build          # cargo build --workspace
make test           # cargo test --workspace (JWT, password hashing, ...)
make clippy         # cargo clippy --workspace --all-targets
make up             # run the full stack via docker-compose
make smoke          # end-to-end smoke test
make down           # stop + remove volumes
```

> The services use sqlx **runtime-checked** queries (`query_as`/`query_scalar`),
> so no live database or `.sqlx` cache is needed to compile — the Docker build
> is fully offline.

## Making changes

1. **Branch** from `main`: `git checkout -b feat/short-description`.
2. If you change `proto/**`, the `proto` crate regenerates on the next build
   (`tonic-build` in `crates/proto/build.rs`).
3. Keep code idiomatic; run `cargo fmt` and `cargo clippy`.
4. Add/adjust tests where it makes sense (`make test` must pass).
5. Verify end-to-end: `make up && make smoke`.

## Commit messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(auth): add RevokeRole RPC and REST route
fix(gateway): map FailedPrecondition to HTTP 409
docs(rbac): document role:write permission
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `build`, `ci`.

## Pull requests

- Keep PRs focused and reasonably small.
- Describe **what** and **why**; link any related issue.
- Ensure `make build`, `make test`, and `make smoke` pass.
- Update docs (`docs/en` **and** `docs/id`) when behavior or APIs change.

By contributing you agree your contributions are licensed under the project's
[MIT License](LICENSE).

---

## Berkontribusi (Bahasa Indonesia)

🌐 [English](#contributing-to-iam-rust) | **Bahasa Indonesia**

Terima kasih atas minatnya untuk berkontribusi!

### Prasyarat

- Rust (stable, edition 2021) + Cargo
- `protobuf-compiler` (`protoc`) — dibutuhkan `tonic-build` untuk build lokal
- Docker + Docker Compose (untuk stack penuh; build Docker tidak butuh toolchain Rust lokal)

### Penyiapan lokal

```bash
make build          # cargo build --workspace
make test           # cargo test --workspace
make clippy         # cargo clippy
make up             # jalankan stack via docker-compose
make smoke          # smoke test end-to-end
make down           # hentikan + hapus volume
```

> Service memakai query sqlx **runtime-checked**, jadi tidak butuh database hidup
> atau cache `.sqlx` untuk meng-compile — build Docker sepenuhnya offline.

### Membuat perubahan

1. **Branch** dari `main`: `git checkout -b feat/deskripsi-singkat`.
2. Jika mengubah `proto/**`, crate `proto` di-regen otomatis saat build
   (`tonic-build` di `crates/proto/build.rs`).
3. Jaga kode idiomatik; jalankan `cargo fmt` dan `cargo clippy`.
4. Tambah/sesuaikan test bila perlu (`make test` harus lulus).
5. Verifikasi end-to-end: `make up && make smoke`.

### Pesan commit

Mengikuti [Conventional Commits](https://www.conventionalcommits.org/). Tipe:
`feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `build`, `ci`.

### Pull request

- Fokus dan tidak terlalu besar.
- Jelaskan **apa** dan **kenapa**; tautkan issue terkait.
- Pastikan `make build`, `make test`, dan `make smoke` lulus.
- Perbarui dokumentasi (`docs/en` **dan** `docs/id`) saat perilaku/API berubah.

Dengan berkontribusi, kamu setuju kontribusimu dilisensikan di bawah
[Lisensi MIT](LICENSE) project ini.
