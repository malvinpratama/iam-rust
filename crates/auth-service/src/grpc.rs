//! Tonic implementation of AuthService.

use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use common::jwt::JwtManager;
use common::password;
use proto::auth::v1::auth_service_server::AuthService;
use proto::auth::v1::*;

use crate::repo::Repo;

const DEFAULT_ROLE: &str = "user";

pub struct AuthSvc {
    repo: Repo,
    jwt: JwtManager,
    refresh_ttl_secs: i64,
    dummy_hash: String, // constant-time login on unknown users
}

/// Enforce a permission from the gateway-supplied identity metadata
/// (defense-in-depth: the service re-checks, not just the gateway).
fn require_perm(md: &tonic::metadata::MetadataMap, perm: &str) -> Result<(), Status> {
    let ok = md
        .get("x-user-permissions")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').any(|p| p == perm))
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(Status::permission_denied(format!("permission denied: {perm}")))
    }
}

impl AuthSvc {
    pub fn new(repo: Repo, jwt: JwtManager, refresh_ttl_secs: i64) -> Self {
        let dummy_hash = password::hash("constant-time-dummy-password").unwrap_or_default();
        Self { repo, jwt, refresh_ttl_secs, dummy_hash }
    }

    async fn issue_tokens(&self, user_id: Uuid, email: &str) -> Result<TokenPair, Status> {
        let access = self
            .jwt
            .issue(&user_id.to_string(), email)
            .map_err(|_| Status::internal("failed to sign token"))?;
        let refresh = gen_refresh_token();
        let expires = Utc::now() + Duration::seconds(self.refresh_ttl_secs);
        self.repo
            .create_refresh_token(user_id, &hash_token(&refresh), expires)
            .await
            .map_err(|_| Status::internal("failed to persist refresh token"))?;
        Ok(TokenPair {
            access_token: access,
            refresh_token: refresh,
            expires_in: self.jwt.access_ttl_secs(),
            token_type: "Bearer".into(),
        })
    }
}

#[tonic::async_trait]
impl AuthService for AuthSvc {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        if req.email.is_empty() || req.password.is_empty() {
            return Err(Status::invalid_argument("email and password are required"));
        }
        let hash =
            password::hash(&req.password).map_err(|_| Status::internal("failed to hash password"))?;
        let id = self
            .repo
            .create_user_with_role(&req.email, &hash, DEFAULT_ROLE)
            .await
            .map_err(|_| Status::already_exists("email already registered"))?;
        Ok(Response::new(RegisterResponse {
            user_id: id.to_string(),
            email: req.email,
        }))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<TokenPair>, Status> {
        let req = request.into_inner();
        let user = match self
            .repo
            .get_user_by_email(&req.email)
            .await
            .map_err(|_| Status::internal("db error"))?
        {
            Some(u) => u,
            None => {
                // Unknown user: still run a verify so timing doesn't leak existence.
                let _ = password::verify(&self.dummy_hash, &req.password);
                return Err(Status::unauthenticated("invalid credentials"));
            }
        };
        if !password::verify(&user.password_hash, &req.password) {
            return Err(Status::unauthenticated("invalid credentials"));
        }
        let pair = self.issue_tokens(user.id, &user.email).await?;
        Ok(Response::new(pair))
    }

    async fn refresh(
        &self,
        request: Request<RefreshRequest>,
    ) -> Result<Response<TokenPair>, Status> {
        let req = request.into_inner();
        let hash = hash_token(&req.refresh_token);
        let row = self
            .repo
            .get_refresh_token(&hash)
            .await
            .map_err(|_| Status::internal("db error"))?
            .ok_or_else(|| Status::unauthenticated("invalid refresh token"))?;
        if row.revoked_at.is_some() {
            return Err(Status::unauthenticated("refresh token revoked"));
        }
        if row.expires_at < Utc::now() {
            return Err(Status::unauthenticated("refresh token expired"));
        }
        let user = self
            .repo
            .get_user_by_id(row.user_id)
            .await
            .map_err(|_| Status::internal("db error"))?
            .ok_or_else(|| Status::unauthenticated("user not found"))?;
        // Rotate: revoke the presented token, issue a fresh pair.
        self.repo
            .revoke_refresh_token(&hash)
            .await
            .map_err(|_| Status::internal("failed to rotate token"))?;
        let pair = self.issue_tokens(user.id, &user.email).await?;
        Ok(Response::new(pair))
    }

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        let req = request.into_inner();
        self.repo
            .revoke_refresh_token(&hash_token(&req.refresh_token))
            .await
            .map_err(|_| Status::internal("failed to revoke token"))?;
        // Best-effort: denylist the access token (by jti) so it stops working now.
        if !req.access_token.is_empty() {
            if let Ok(claims) = self.jwt.parse(&req.access_token) {
                if let Some(exp) = chrono::DateTime::from_timestamp(claims.exp, 0) {
                    let _ = self.repo.revoke_access_jti(&claims.jti, exp).await;
                }
            }
        }
        Ok(Response::new(LogoutResponse { success: true }))
    }

    async fn validate_token(
        &self,
        request: Request<ValidateTokenRequest>,
    ) -> Result<Response<ValidateTokenResponse>, Status> {
        let req = request.into_inner();
        let claims = self
            .jwt
            .parse(&req.access_token)
            .map_err(|_| Status::unauthenticated("invalid or expired token"))?;
        if self
            .repo
            .is_token_revoked(&claims.jti)
            .await
            .map_err(|_| Status::internal("failed to check token status"))?
        {
            return Err(Status::unauthenticated("token revoked"));
        }
        let user_id =
            Uuid::parse_str(&claims.sub).map_err(|_| Status::unauthenticated("invalid subject"))?;
        let roles = self
            .repo
            .get_user_roles(user_id)
            .await
            .map_err(|_| Status::internal("failed to load roles"))?;
        let permissions = self
            .repo
            .get_user_permissions(user_id)
            .await
            .map_err(|_| Status::internal("failed to load permissions"))?;
        Ok(Response::new(ValidateTokenResponse {
            user_id: claims.sub,
            email: claims.email,
            roles,
            permissions,
        }))
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        require_perm(request.metadata(), "user:delete")?;
        let req = request.into_inner();
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("invalid user id"))?;
        self.repo
            .delete_user(user_id)
            .await
            .map_err(|_| Status::internal("failed to delete user"))?;
        Ok(Response::new(DeleteUserResponse { success: true }))
    }

    async fn create_role(
        &self,
        request: Request<CreateRoleRequest>,
    ) -> Result<Response<Role>, Status> {
        require_perm(request.metadata(), "role:write")?;
        let req = request.into_inner();
        let role = self
            .repo
            .create_role(&req.name, &req.description)
            .await
            .map_err(|_| Status::already_exists("role already exists"))?;
        Ok(Response::new(Role {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: vec![],
        }))
    }

    async fn update_role(
        &self,
        request: Request<UpdateRoleRequest>,
    ) -> Result<Response<Role>, Status> {
        require_perm(request.metadata(), "role:write")?;
        let req = request.into_inner();
        let role = self
            .repo
            .update_role(&req.name, &req.description)
            .await
            .map_err(|_| Status::internal("db error"))?
            .ok_or_else(|| Status::not_found("role not found"))?;
        Ok(Response::new(Role {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: vec![],
        }))
    }

    async fn delete_role(
        &self,
        request: Request<DeleteRoleRequest>,
    ) -> Result<Response<DeleteRoleResponse>, Status> {
        require_perm(request.metadata(), "role:write")?;
        let req = request.into_inner();
        if req.name == "admin" || req.name == "user" {
            return Err(Status::failed_precondition("cannot delete built-in role"));
        }
        if !self
            .repo
            .role_exists(&req.name)
            .await
            .map_err(|_| Status::internal("db error"))?
        {
            return Err(Status::not_found("role not found"));
        }
        self.repo
            .delete_role(&req.name)
            .await
            .map_err(|_| Status::internal("failed to delete role"))?;
        Ok(Response::new(DeleteRoleResponse { success: true }))
    }

    async fn list_roles(
        &self,
        _request: Request<ListRolesRequest>,
    ) -> Result<Response<ListRolesResponse>, Status> {
        let rows = self
            .repo
            .list_roles()
            .await
            .map_err(|_| Status::internal("failed to list roles"))?;
        let mut roles = Vec::with_capacity(rows.len());
        for r in rows {
            let perms = self
                .repo
                .list_role_permissions(r.id)
                .await
                .map_err(|_| Status::internal("failed to list role permissions"))?;
            roles.push(Role {
                id: r.id,
                name: r.name,
                description: r.description,
                permissions: perms,
            });
        }
        Ok(Response::new(ListRolesResponse { roles }))
    }

    async fn assign_role(
        &self,
        request: Request<AssignRoleRequest>,
    ) -> Result<Response<AssignRoleResponse>, Status> {
        require_perm(request.metadata(), "role:assign")?;
        let req = request.into_inner();
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("invalid user id"))?;
        if !self
            .repo
            .role_exists(&req.role_name)
            .await
            .map_err(|_| Status::internal("db error"))?
        {
            return Err(Status::not_found("role not found"));
        }
        self.repo
            .assign_role(user_id, &req.role_name)
            .await
            .map_err(|_| Status::internal("failed to assign role"))?;
        Ok(Response::new(AssignRoleResponse { success: true }))
    }

    async fn revoke_role(
        &self,
        request: Request<RevokeRoleRequest>,
    ) -> Result<Response<RevokeRoleResponse>, Status> {
        require_perm(request.metadata(), "role:assign")?;
        let req = request.into_inner();
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("invalid user id"))?;
        if !self
            .repo
            .role_exists(&req.role_name)
            .await
            .map_err(|_| Status::internal("db error"))?
        {
            return Err(Status::not_found("role not found"));
        }
        self.repo
            .revoke_role(user_id, &req.role_name)
            .await
            .map_err(|_| Status::internal("failed to revoke role"))?;
        Ok(Response::new(RevokeRoleResponse { success: true }))
    }

    async fn list_permissions(
        &self,
        _request: Request<ListPermissionsRequest>,
    ) -> Result<Response<ListPermissionsResponse>, Status> {
        let rows = self
            .repo
            .list_permissions()
            .await
            .map_err(|_| Status::internal("failed to list permissions"))?;
        let permissions = rows
            .into_iter()
            .map(|p| Permission {
                id: p.id,
                name: p.name,
                description: p.description,
            })
            .collect();
        Ok(Response::new(ListPermissionsResponse { permissions }))
    }

    async fn grant_permission(
        &self,
        request: Request<GrantPermissionRequest>,
    ) -> Result<Response<GrantPermissionResponse>, Status> {
        require_perm(request.metadata(), "role:write")?;
        let req = request.into_inner();
        self.repo
            .grant_permission(&req.role_name, &req.permission_name)
            .await
            .map_err(|_| Status::internal("failed to grant permission"))?;
        Ok(Response::new(GrantPermissionResponse { success: true }))
    }

    async fn revoke_permission(
        &self,
        request: Request<RevokePermissionRequest>,
    ) -> Result<Response<RevokePermissionResponse>, Status> {
        require_perm(request.metadata(), "role:write")?;
        let req = request.into_inner();
        self.repo
            .revoke_permission(&req.role_name, &req.permission_name)
            .await
            .map_err(|_| Status::internal("failed to revoke permission"))?;
        Ok(Response::new(RevokePermissionResponse { success: true }))
    }
}

fn gen_refresh_token() -> String {
    let mut b = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut b);
    hex::encode(b)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
