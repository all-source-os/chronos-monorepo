use async_trait::async_trait;
use better_auth_core::adapters::traits::{
    AccountOps, ApiKeyOps, InvitationOps, MemberOps, OrganizationOps, PasskeyOps, SessionOps,
    TwoFactorOps, UserOps, VerificationOps,
};
use better_auth_core::error::{AuthError, AuthResult};
use better_auth_core::types::{
    Account, ApiKey, CreateAccount, CreateApiKey, CreateInvitation, CreateMember,
    CreateOrganization, CreatePasskey, CreateSession, CreateTwoFactor, CreateUser,
    CreateVerification, Invitation, InvitationStatus, ListUsersParams, Member, Organization,
    Passkey, Session, TwoFactor, UpdateAccount, UpdateApiKey, UpdateOrganization, UpdateUser, User,
    Verification,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::client::AllsourceClient;

/// Allsource-backed `DatabaseAdapter` for better-auth-rs.
///
/// Each auth entity is stored as an Allsource event stream:
/// - Entity ID pattern: `auth-{type}:{id}` (e.g., `auth-user:abc123`)
/// - Each create/update appends a full-state snapshot event
/// - Deletes append a `_deleted: true` marker
/// - Reads fetch the latest event and deserialize the payload
///
/// This gives you event-sourced auth with full history and time-travel.
pub struct AllsourceAuthAdapter {
    client: AllsourceClient,
}

impl AllsourceAuthAdapter {
    /// Create a new adapter.
    ///
    /// - `core_url`: Allsource Core URL (e.g., `http://localhost:3900`)
    /// - `query_url`: Allsource Query Service URL (e.g., `http://localhost:3902`)
    /// - `api_key`: API key for authentication (e.g., `ask_...`)
    pub fn new(core_url: &str, query_url: &str, api_key: &str) -> Self {
        Self {
            client: AllsourceClient::new(core_url, query_url, api_key),
        }
    }
}

// Entity ID helpers
fn user_entity(id: &str) -> String {
    format!("auth-user:{id}")
}
fn session_entity(token: &str) -> String {
    format!("auth-session:{token}")
}
fn account_entity(id: &str) -> String {
    format!("auth-account:{id}")
}
fn verification_entity(id: &str) -> String {
    format!("auth-verification:{id}")
}
fn org_entity(id: &str) -> String {
    format!("auth-org:{id}")
}
fn member_entity(id: &str) -> String {
    format!("auth-member:{id}")
}
fn invitation_entity(id: &str) -> String {
    format!("auth-invitation:{id}")
}
fn two_factor_entity(id: &str) -> String {
    format!("auth-2fa:{id}")
}
fn api_key_entity(id: &str) -> String {
    format!("auth-apikey:{id}")
}
fn passkey_entity(id: &str) -> String {
    format!("auth-passkey:{id}")
}

// ── UserOps ──

#[async_trait]
impl UserOps for AllsourceAuthAdapter {
    type User = User;

    async fn create_user(&self, input: CreateUser) -> AuthResult<User> {
        let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

        // Check email uniqueness
        if let Some(email) = &input.email {
            if let Some(_existing) = self
                .client
                .find_by_field::<User>("auth.user.created", "email", email)
                .await
                .map_err(AuthError::from)?
            {
                return Err(AuthError::config("Email already exists"));
            }
        }

        // Check username uniqueness
        if let Some(username) = &input.username {
            if let Some(_existing) = self
                .client
                .find_by_field::<User>("auth.user.created", "username", username)
                .await
                .map_err(AuthError::from)?
            {
                return Err(AuthError::conflict(
                    "A user with this username already exists",
                ));
            }
        }

        let now = Utc::now();
        let user = User {
            id: id.clone(),
            email: input.email,
            name: input.name,
            image: input.image,
            email_verified: input.email_verified.unwrap_or(false),
            created_at: now,
            updated_at: now,
            username: input.username,
            display_username: input.display_username,
            two_factor_enabled: false,
            role: input.role,
            banned: false,
            ban_reason: None,
            ban_expires: None,
            metadata: input.metadata.unwrap_or(serde_json::Value::Null),
        };

        let payload = serde_json::to_value(&user).map_err(AuthError::from)?;
        self.client
            .append_event(&user_entity(&id), "auth.user.created", payload)
            .await
            .map_err(AuthError::from)?;

        // Store password in the account, not the user (matches better-auth pattern)
        // Password is handled via AccountOps

        Ok(user)
    }

    async fn get_user_by_id(&self, id: &str) -> AuthResult<Option<User>> {
        self.client
            .get_latest(&user_entity(id))
            .await
            .map_err(AuthError::from)
    }

    async fn get_user_by_email(&self, email: &str) -> AuthResult<Option<User>> {
        self.client
            .find_by_field("auth.user.created", "email", email)
            .await
            .map_err(AuthError::from)
    }

    async fn get_user_by_username(&self, username: &str) -> AuthResult<Option<User>> {
        self.client
            .find_by_field("auth.user.created", "username", username)
            .await
            .map_err(AuthError::from)
    }

    async fn update_user(&self, id: &str, update: UpdateUser) -> AuthResult<User> {
        let mut user = self
            .get_user_by_id(id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        if let Some(email) = update.email {
            user.email = Some(email);
        }
        if let Some(name) = update.name {
            user.name = Some(name);
        }
        if let Some(image) = update.image {
            user.image = Some(image);
        }
        if let Some(verified) = update.email_verified {
            user.email_verified = verified;
        }
        if let Some(username) = update.username {
            user.username = Some(username);
        }
        if let Some(display_username) = update.display_username {
            user.display_username = Some(display_username);
        }
        if let Some(role) = update.role {
            user.role = Some(role);
        }
        if let Some(banned) = update.banned {
            user.banned = banned;
        }
        if let Some(ban_reason) = update.ban_reason {
            user.ban_reason = Some(ban_reason);
        }
        if let Some(ban_expires) = update.ban_expires {
            user.ban_expires = Some(ban_expires);
        }
        if let Some(two_factor_enabled) = update.two_factor_enabled {
            user.two_factor_enabled = two_factor_enabled;
        }
        if let Some(metadata) = update.metadata {
            user.metadata = metadata;
        }
        user.updated_at = Utc::now();

        let payload = serde_json::to_value(&user).map_err(AuthError::from)?;
        self.client
            .append_event(&user_entity(id), "auth.user.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(user)
    }

    async fn delete_user(&self, id: &str) -> AuthResult<()> {
        self.client
            .append_delete(&user_entity(id), "auth.user.deleted")
            .await
            .map_err(AuthError::from)
    }

    async fn list_users(&self, params: ListUsersParams) -> AuthResult<(Vec<User>, usize)> {
        let mut users: Vec<User> = self
            .client
            .query_all("auth.user.created", 10000)
            .await
            .map_err(AuthError::from)?;

        // Apply search filter
        if let Some(search_value) = &params.search_value {
            let field = params.search_field.as_deref().unwrap_or("email");
            let op = params.search_operator.as_deref().unwrap_or("contains");
            let sv = search_value.to_lowercase();
            users.retain(|u| {
                let field_val = match field {
                    "name" => u.name.as_deref().unwrap_or("").to_lowercase(),
                    _ => u.email.as_deref().unwrap_or("").to_lowercase(),
                };
                match op {
                    "starts_with" => field_val.starts_with(&sv),
                    "ends_with" => field_val.ends_with(&sv),
                    _ => field_val.contains(&sv),
                }
            });
        }

        // Apply filter
        if let Some(filter_value) = &params.filter_value {
            let field = params.filter_field.as_deref().unwrap_or("email");
            let op = params.filter_operator.as_deref().unwrap_or("eq");
            let fv = filter_value.to_lowercase();
            users.retain(|u| {
                let field_val = match field {
                    "name" => u.name.as_deref().unwrap_or("").to_lowercase(),
                    "role" => u.role.as_deref().unwrap_or("").to_lowercase(),
                    _ => u.email.as_deref().unwrap_or("").to_lowercase(),
                };
                match op {
                    "contains" => field_val.contains(&fv),
                    "starts_with" => field_val.starts_with(&fv),
                    "ends_with" => field_val.ends_with(&fv),
                    "ne" => field_val != fv,
                    _ => field_val == fv,
                }
            });
        }

        let total = users.len();
        let offset = params.offset.unwrap_or(0);
        let limit = params.limit.unwrap_or(100);
        let paged: Vec<User> = users.into_iter().skip(offset).take(limit).collect();

        Ok((paged, total))
    }
}

// ── SessionOps ──

#[async_trait]
impl SessionOps for AllsourceAuthAdapter {
    type Session = Session;

    async fn create_session(&self, input: CreateSession) -> AuthResult<Session> {
        let id = Uuid::new_v4().to_string();
        let token = format!("session_{}", Uuid::new_v4());
        let now = Utc::now();

        let session = Session {
            id,
            token: token.clone(),
            user_id: input.user_id,
            expires_at: input.expires_at,
            created_at: now,
            updated_at: now,
            ip_address: input.ip_address,
            user_agent: input.user_agent,
            impersonated_by: input.impersonated_by,
            active_organization_id: input.active_organization_id,
            active: true,
        };

        let payload = serde_json::to_value(&session).map_err(AuthError::from)?;
        self.client
            .append_event(&session_entity(&token), "auth.session.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(session)
    }

    async fn get_session(&self, token: &str) -> AuthResult<Option<Session>> {
        self.client
            .get_latest(&session_entity(token))
            .await
            .map_err(AuthError::from)
    }

    async fn get_user_sessions(&self, user_id: &str) -> AuthResult<Vec<Session>> {
        self.client
            .find_all_by_field("auth.session.created", "user_id", user_id)
            .await
            .map_err(AuthError::from)
    }

    async fn update_session_expiry(
        &self,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> AuthResult<()> {
        if let Some(mut session) = self.get_session(token).await? {
            session.expires_at = expires_at;
            session.updated_at = Utc::now();
            let payload = serde_json::to_value(&session).map_err(AuthError::from)?;
            self.client
                .append_event(&session_entity(token), "auth.session.expiry_updated", payload)
                .await
                .map_err(AuthError::from)?;
        }
        Ok(())
    }

    async fn delete_session(&self, token: &str) -> AuthResult<()> {
        self.client
            .append_delete(&session_entity(token), "auth.session.deleted")
            .await
            .map_err(AuthError::from)
    }

    async fn delete_user_sessions(&self, user_id: &str) -> AuthResult<()> {
        let sessions = self.get_user_sessions(user_id).await?;
        for session in sessions {
            self.delete_session(&session.token).await?;
        }
        Ok(())
    }

    async fn delete_expired_sessions(&self) -> AuthResult<usize> {
        let all: Vec<Session> = self
            .client
            .query_all("auth.session.created", 10000)
            .await
            .map_err(AuthError::from)?;

        let now = Utc::now();
        let mut deleted = 0;
        for session in all {
            if session.expires_at <= now || !session.active {
                self.delete_session(&session.token).await?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    async fn update_session_active_organization(
        &self,
        token: &str,
        organization_id: Option<&str>,
    ) -> AuthResult<Session> {
        let mut session = self
            .get_session(token)
            .await?
            .ok_or(AuthError::SessionNotFound)?;

        session.active_organization_id = organization_id.map(|s| s.to_string());
        session.updated_at = Utc::now();

        let payload = serde_json::to_value(&session).map_err(AuthError::from)?;
        self.client
            .append_event(&session_entity(token), "auth.session.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(session)
    }
}

// ── AccountOps ──

#[async_trait]
impl AccountOps for AllsourceAuthAdapter {
    type Account = Account;

    async fn create_account(&self, input: CreateAccount) -> AuthResult<Account> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let account = Account {
            id: id.clone(),
            account_id: input.account_id,
            provider_id: input.provider_id,
            user_id: input.user_id,
            access_token: input.access_token,
            refresh_token: input.refresh_token,
            id_token: input.id_token,
            access_token_expires_at: input.access_token_expires_at,
            refresh_token_expires_at: input.refresh_token_expires_at,
            scope: input.scope,
            password: input.password,
            created_at: now,
            updated_at: now,
        };

        let payload = serde_json::to_value(&account).map_err(AuthError::from)?;
        self.client
            .append_event(&account_entity(&id), "auth.account.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(account)
    }

    async fn get_account(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> AuthResult<Option<Account>> {
        // Need to scan accounts matching provider + account_id
        let all: Vec<Account> = self
            .client
            .query_all("auth.account.created", 10000)
            .await
            .map_err(AuthError::from)?;

        Ok(all
            .into_iter()
            .find(|a| a.provider_id == provider && a.account_id == provider_account_id))
    }

    async fn get_user_accounts(&self, user_id: &str) -> AuthResult<Vec<Account>> {
        self.client
            .find_all_by_field("auth.account.created", "user_id", user_id)
            .await
            .map_err(AuthError::from)
    }

    async fn update_account(&self, id: &str, update: UpdateAccount) -> AuthResult<Account> {
        let mut account: Account = self
            .client
            .get_latest(&account_entity(id))
            .await
            .map_err(AuthError::from)?
            .ok_or_else(|| AuthError::not_found("Account not found"))?;

        if let Some(access_token) = update.access_token {
            account.access_token = Some(access_token);
        }
        if let Some(refresh_token) = update.refresh_token {
            account.refresh_token = Some(refresh_token);
        }
        if let Some(id_token) = update.id_token {
            account.id_token = Some(id_token);
        }
        if let Some(expires_at) = update.access_token_expires_at {
            account.access_token_expires_at = Some(expires_at);
        }
        if let Some(expires_at) = update.refresh_token_expires_at {
            account.refresh_token_expires_at = Some(expires_at);
        }
        if let Some(scope) = update.scope {
            account.scope = Some(scope);
        }
        if let Some(password) = update.password {
            account.password = Some(password);
        }
        account.updated_at = Utc::now();

        let payload = serde_json::to_value(&account).map_err(AuthError::from)?;
        self.client
            .append_event(&account_entity(id), "auth.account.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(account)
    }

    async fn delete_account(&self, id: &str) -> AuthResult<()> {
        self.client
            .append_delete(&account_entity(id), "auth.account.deleted")
            .await
            .map_err(AuthError::from)
    }
}

// ── VerificationOps ──

#[async_trait]
impl VerificationOps for AllsourceAuthAdapter {
    type Verification = Verification;

    async fn create_verification(&self, input: CreateVerification) -> AuthResult<Verification> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let verification = Verification {
            id: id.clone(),
            identifier: input.identifier,
            value: input.value,
            expires_at: input.expires_at,
            created_at: now,
            updated_at: now,
        };

        let payload = serde_json::to_value(&verification).map_err(AuthError::from)?;
        self.client
            .append_event(
                &verification_entity(&id),
                "auth.verification.created",
                payload,
            )
            .await
            .map_err(AuthError::from)?;

        Ok(verification)
    }

    async fn get_verification(
        &self,
        identifier: &str,
        value: &str,
    ) -> AuthResult<Option<Verification>> {
        let all: Vec<Verification> = self
            .client
            .query_all("auth.verification.created", 10000)
            .await
            .map_err(AuthError::from)?;

        let now = Utc::now();
        Ok(all
            .into_iter()
            .find(|v| v.identifier == identifier && v.value == value && v.expires_at > now))
    }

    async fn get_verification_by_value(&self, value: &str) -> AuthResult<Option<Verification>> {
        let all: Vec<Verification> = self
            .client
            .query_all("auth.verification.created", 10000)
            .await
            .map_err(AuthError::from)?;

        let now = Utc::now();
        Ok(all
            .into_iter()
            .find(|v| v.value == value && v.expires_at > now))
    }

    async fn get_verification_by_identifier(
        &self,
        identifier: &str,
    ) -> AuthResult<Option<Verification>> {
        let all: Vec<Verification> = self
            .client
            .query_all("auth.verification.created", 10000)
            .await
            .map_err(AuthError::from)?;

        let now = Utc::now();
        Ok(all
            .into_iter()
            .find(|v| v.identifier == identifier && v.expires_at > now))
    }

    async fn consume_verification(
        &self,
        identifier: &str,
        value: &str,
    ) -> AuthResult<Option<Verification>> {
        let verification = self.get_verification(identifier, value).await?;
        if let Some(ref v) = verification {
            self.client
                .append_delete(&verification_entity(&v.id), "auth.verification.consumed")
                .await
                .map_err(AuthError::from)?;
        }
        Ok(verification)
    }

    async fn delete_verification(&self, id: &str) -> AuthResult<()> {
        self.client
            .append_delete(&verification_entity(id), "auth.verification.deleted")
            .await
            .map_err(AuthError::from)
    }

    async fn delete_expired_verifications(&self) -> AuthResult<usize> {
        let all: Vec<Verification> = self
            .client
            .query_all("auth.verification.created", 10000)
            .await
            .map_err(AuthError::from)?;

        let now = Utc::now();
        let mut deleted = 0;
        for v in all {
            if v.expires_at <= now {
                self.delete_verification(&v.id).await?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

// ── OrganizationOps ──

#[async_trait]
impl OrganizationOps for AllsourceAuthAdapter {
    type Organization = Organization;

    async fn create_organization(&self, input: CreateOrganization) -> AuthResult<Organization> {
        // Check slug uniqueness
        if self
            .client
            .find_by_field::<Organization>("auth.org.created", "slug", &input.slug)
            .await
            .map_err(AuthError::from)?
            .is_some()
        {
            return Err(AuthError::conflict("Organization slug already exists"));
        }

        let id = input
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();

        let org = Organization {
            id: id.clone(),
            name: input.name,
            slug: input.slug,
            logo: input.logo,
            metadata: input.metadata,
            created_at: now,
            updated_at: now,
        };

        let payload = serde_json::to_value(&org).map_err(AuthError::from)?;
        self.client
            .append_event(&org_entity(&id), "auth.org.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(org)
    }

    async fn get_organization_by_id(&self, id: &str) -> AuthResult<Option<Organization>> {
        self.client
            .get_latest(&org_entity(id))
            .await
            .map_err(AuthError::from)
    }

    async fn get_organization_by_slug(&self, slug: &str) -> AuthResult<Option<Organization>> {
        self.client
            .find_by_field("auth.org.created", "slug", slug)
            .await
            .map_err(AuthError::from)
    }

    async fn update_organization(
        &self,
        id: &str,
        update: UpdateOrganization,
    ) -> AuthResult<Organization> {
        let mut org: Organization = self
            .get_organization_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("Organization not found"))?;

        if let Some(name) = update.name {
            org.name = name;
        }
        if let Some(slug) = update.slug {
            org.slug = slug;
        }
        if let Some(logo) = update.logo {
            org.logo = Some(logo);
        }
        if let Some(metadata) = update.metadata {
            org.metadata = Some(metadata);
        }
        org.updated_at = Utc::now();

        let payload = serde_json::to_value(&org).map_err(AuthError::from)?;
        self.client
            .append_event(&org_entity(id), "auth.org.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(org)
    }

    async fn delete_organization(&self, id: &str) -> AuthResult<()> {
        self.client
            .append_delete(&org_entity(id), "auth.org.deleted")
            .await
            .map_err(AuthError::from)?;

        // Also delete members and invitations for this org
        let members: Vec<Member> = self
            .client
            .find_all_by_field("auth.member.created", "organization_id", id)
            .await
            .map_err(AuthError::from)?;
        for m in members {
            self.client
                .append_delete(&member_entity(&m.id), "auth.member.deleted")
                .await
                .map_err(AuthError::from)?;
        }

        let invitations: Vec<Invitation> = self
            .client
            .find_all_by_field("auth.invitation.created", "organization_id", id)
            .await
            .map_err(AuthError::from)?;
        for i in invitations {
            self.client
                .append_delete(&invitation_entity(&i.id), "auth.invitation.deleted")
                .await
                .map_err(AuthError::from)?;
        }

        Ok(())
    }

    async fn list_user_organizations(&self, user_id: &str) -> AuthResult<Vec<Organization>> {
        let members: Vec<Member> = self
            .client
            .find_all_by_field("auth.member.created", "user_id", user_id)
            .await
            .map_err(AuthError::from)?;

        let mut orgs = Vec::new();
        for m in members {
            if let Some(org) = self.get_organization_by_id(&m.organization_id).await? {
                orgs.push(org);
            }
        }
        Ok(orgs)
    }
}

// ── MemberOps ──

#[async_trait]
impl MemberOps for AllsourceAuthAdapter {
    type Member = Member;

    async fn create_member(&self, input: CreateMember) -> AuthResult<Member> {
        // Check if already a member
        let existing: Vec<Member> = self
            .client
            .find_all_by_field("auth.member.created", "organization_id", &input.organization_id)
            .await
            .map_err(AuthError::from)?;

        if existing.iter().any(|m| m.user_id == input.user_id) {
            return Err(AuthError::conflict(
                "User is already a member of this organization",
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let member = Member {
            id: id.clone(),
            organization_id: input.organization_id,
            user_id: input.user_id,
            role: input.role,
            created_at: now,
        };

        let payload = serde_json::to_value(&member).map_err(AuthError::from)?;
        self.client
            .append_event(&member_entity(&id), "auth.member.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(member)
    }

    async fn get_member(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> AuthResult<Option<Member>> {
        let members: Vec<Member> = self
            .client
            .find_all_by_field("auth.member.created", "organization_id", organization_id)
            .await
            .map_err(AuthError::from)?;

        Ok(members.into_iter().find(|m| m.user_id == user_id))
    }

    async fn get_member_by_id(&self, id: &str) -> AuthResult<Option<Member>> {
        self.client
            .get_latest(&member_entity(id))
            .await
            .map_err(AuthError::from)
    }

    async fn update_member_role(&self, member_id: &str, role: &str) -> AuthResult<Member> {
        let mut member: Member = self
            .get_member_by_id(member_id)
            .await?
            .ok_or_else(|| AuthError::not_found("Member not found"))?;

        member.role = role.to_string();

        let payload = serde_json::to_value(&member).map_err(AuthError::from)?;
        self.client
            .append_event(&member_entity(member_id), "auth.member.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(member)
    }

    async fn delete_member(&self, member_id: &str) -> AuthResult<()> {
        self.client
            .append_delete(&member_entity(member_id), "auth.member.deleted")
            .await
            .map_err(AuthError::from)
    }

    async fn list_organization_members(&self, organization_id: &str) -> AuthResult<Vec<Member>> {
        self.client
            .find_all_by_field("auth.member.created", "organization_id", organization_id)
            .await
            .map_err(AuthError::from)
    }

    async fn count_organization_members(&self, organization_id: &str) -> AuthResult<usize> {
        Ok(self.list_organization_members(organization_id).await?.len())
    }

    async fn count_organization_owners(&self, organization_id: &str) -> AuthResult<usize> {
        let members = self.list_organization_members(organization_id).await?;
        Ok(members.iter().filter(|m| m.role == "owner").count())
    }
}

// ── InvitationOps ──

#[async_trait]
impl InvitationOps for AllsourceAuthAdapter {
    type Invitation = Invitation;

    async fn create_invitation(&self, input: CreateInvitation) -> AuthResult<Invitation> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let invitation = Invitation {
            id: id.clone(),
            organization_id: input.organization_id,
            email: input.email,
            role: input.role,
            status: InvitationStatus::Pending,
            inviter_id: input.inviter_id,
            expires_at: input.expires_at,
            created_at: now,
        };

        let payload = serde_json::to_value(&invitation).map_err(AuthError::from)?;
        self.client
            .append_event(&invitation_entity(&id), "auth.invitation.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(invitation)
    }

    async fn get_invitation_by_id(&self, id: &str) -> AuthResult<Option<Invitation>> {
        self.client
            .get_latest(&invitation_entity(id))
            .await
            .map_err(AuthError::from)
    }

    async fn get_pending_invitation(
        &self,
        organization_id: &str,
        email: &str,
    ) -> AuthResult<Option<Invitation>> {
        let invitations: Vec<Invitation> = self
            .client
            .find_all_by_field("auth.invitation.created", "organization_id", organization_id)
            .await
            .map_err(AuthError::from)?;

        Ok(invitations.into_iter().find(|i| {
            i.email.to_lowercase() == email.to_lowercase()
                && i.status == InvitationStatus::Pending
        }))
    }

    async fn update_invitation_status(
        &self,
        id: &str,
        status: InvitationStatus,
    ) -> AuthResult<Invitation> {
        let mut invitation = self
            .get_invitation_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("Invitation not found"))?;

        invitation.status = status;

        let payload = serde_json::to_value(&invitation).map_err(AuthError::from)?;
        self.client
            .append_event(&invitation_entity(id), "auth.invitation.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(invitation)
    }

    async fn list_organization_invitations(
        &self,
        organization_id: &str,
    ) -> AuthResult<Vec<Invitation>> {
        self.client
            .find_all_by_field("auth.invitation.created", "organization_id", organization_id)
            .await
            .map_err(AuthError::from)
    }

    async fn list_user_invitations(&self, email: &str) -> AuthResult<Vec<Invitation>> {
        let all: Vec<Invitation> = self
            .client
            .find_all_by_field("auth.invitation.created", "email", email)
            .await
            .map_err(AuthError::from)?;

        let now = Utc::now();
        Ok(all
            .into_iter()
            .filter(|i| i.status == InvitationStatus::Pending && i.expires_at > now)
            .collect())
    }
}

// ── TwoFactorOps ──

#[async_trait]
impl TwoFactorOps for AllsourceAuthAdapter {
    type TwoFactor = TwoFactor;

    async fn create_two_factor(&self, input: CreateTwoFactor) -> AuthResult<TwoFactor> {
        // Check if user already has 2FA
        if self
            .get_two_factor_by_user_id(&input.user_id)
            .await?
            .is_some()
        {
            return Err(AuthError::conflict(
                "Two-factor authentication already enabled for this user",
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let two_factor = TwoFactor {
            id: id.clone(),
            secret: input.secret,
            backup_codes: input.backup_codes,
            user_id: input.user_id,
            created_at: now,
            updated_at: now,
        };

        let payload = serde_json::to_value(&two_factor).map_err(AuthError::from)?;
        self.client
            .append_event(&two_factor_entity(&id), "auth.two_factor.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(two_factor)
    }

    async fn get_two_factor_by_user_id(&self, user_id: &str) -> AuthResult<Option<TwoFactor>> {
        self.client
            .find_by_field("auth.two_factor.created", "user_id", user_id)
            .await
            .map_err(AuthError::from)
    }

    async fn update_two_factor_backup_codes(
        &self,
        user_id: &str,
        backup_codes: &str,
    ) -> AuthResult<TwoFactor> {
        let mut tf = self
            .get_two_factor_by_user_id(user_id)
            .await?
            .ok_or_else(|| AuthError::not_found("Two-factor record not found"))?;

        tf.backup_codes = Some(backup_codes.to_string());
        tf.updated_at = Utc::now();

        let payload = serde_json::to_value(&tf).map_err(AuthError::from)?;
        self.client
            .append_event(&two_factor_entity(&tf.id), "auth.two_factor.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(tf)
    }

    async fn delete_two_factor(&self, user_id: &str) -> AuthResult<()> {
        if let Some(tf) = self.get_two_factor_by_user_id(user_id).await? {
            self.client
                .append_delete(&two_factor_entity(&tf.id), "auth.two_factor.deleted")
                .await
                .map_err(AuthError::from)?;
        }
        Ok(())
    }
}

// ── ApiKeyOps ──

#[async_trait]
impl ApiKeyOps for AllsourceAuthAdapter {
    type ApiKey = ApiKey;

    async fn create_api_key(&self, input: CreateApiKey) -> AuthResult<ApiKey> {
        // Check hash uniqueness
        if self
            .get_api_key_by_hash(&input.key_hash)
            .await?
            .is_some()
        {
            return Err(AuthError::conflict("API key already exists"));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let api_key = ApiKey {
            id: id.clone(),
            name: input.name,
            start: input.start,
            prefix: input.prefix,
            key_hash: input.key_hash,
            user_id: input.user_id,
            refill_interval: input.refill_interval,
            refill_amount: input.refill_amount,
            last_refill_at: None,
            enabled: input.enabled,
            rate_limit_enabled: input.rate_limit_enabled,
            rate_limit_time_window: input.rate_limit_time_window,
            rate_limit_max: input.rate_limit_max,
            request_count: Some(0),
            remaining: input.remaining,
            last_request: None,
            expires_at: input.expires_at,
            created_at: now.clone(),
            updated_at: now,
            permissions: input.permissions,
            metadata: input.metadata,
        };

        let payload = serde_json::to_value(&api_key).map_err(AuthError::from)?;
        self.client
            .append_event(&api_key_entity(&id), "auth.api_key.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(api_key)
    }

    async fn get_api_key_by_id(&self, id: &str) -> AuthResult<Option<ApiKey>> {
        self.client
            .get_latest(&api_key_entity(id))
            .await
            .map_err(AuthError::from)
    }

    async fn get_api_key_by_hash(&self, hash: &str) -> AuthResult<Option<ApiKey>> {
        self.client
            .find_by_field("auth.api_key.created", "key", hash)
            .await
            .map_err(AuthError::from)
    }

    async fn list_api_keys_by_user(&self, user_id: &str) -> AuthResult<Vec<ApiKey>> {
        let mut keys: Vec<ApiKey> = self
            .client
            .find_all_by_field("auth.api_key.created", "user_id", user_id)
            .await
            .map_err(AuthError::from)?;

        keys.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(keys)
    }

    async fn update_api_key(&self, id: &str, update: UpdateApiKey) -> AuthResult<ApiKey> {
        let mut key = self
            .get_api_key_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("API key not found"))?;

        if let Some(name) = update.name {
            key.name = Some(name);
        }
        if let Some(enabled) = update.enabled {
            key.enabled = enabled;
        }
        if let Some(remaining) = update.remaining {
            key.remaining = Some(remaining);
        }
        if let Some(rate_limit_enabled) = update.rate_limit_enabled {
            key.rate_limit_enabled = rate_limit_enabled;
        }
        if let Some(window) = update.rate_limit_time_window {
            key.rate_limit_time_window = Some(window);
        }
        if let Some(max) = update.rate_limit_max {
            key.rate_limit_max = Some(max);
        }
        if let Some(interval) = update.refill_interval {
            key.refill_interval = Some(interval);
        }
        if let Some(amount) = update.refill_amount {
            key.refill_amount = Some(amount);
        }
        if let Some(permissions) = update.permissions {
            key.permissions = Some(permissions);
        }
        if let Some(metadata) = update.metadata {
            key.metadata = Some(metadata);
        }
        if let Some(expires_at) = update.expires_at {
            key.expires_at = expires_at;
        }
        if let Some(last_request) = update.last_request {
            key.last_request = last_request;
        }
        if let Some(request_count) = update.request_count {
            key.request_count = Some(request_count);
        }
        if let Some(last_refill_at) = update.last_refill_at {
            key.last_refill_at = last_refill_at;
        }
        key.updated_at = Utc::now().to_rfc3339();

        let payload = serde_json::to_value(&key).map_err(AuthError::from)?;
        self.client
            .append_event(&api_key_entity(id), "auth.api_key.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(key)
    }

    async fn delete_api_key(&self, id: &str) -> AuthResult<()> {
        self.client
            .append_delete(&api_key_entity(id), "auth.api_key.deleted")
            .await
            .map_err(AuthError::from)
    }

    async fn delete_expired_api_keys(&self) -> AuthResult<usize> {
        let all: Vec<ApiKey> = self
            .client
            .query_all("auth.api_key.created", 10000)
            .await
            .map_err(AuthError::from)?;

        let now = Utc::now();
        let mut deleted = 0;
        for key in all {
            if let Some(expires_at) = &key.expires_at {
                if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                    if exp <= now {
                        self.delete_api_key(&key.id).await?;
                        deleted += 1;
                    }
                }
            }
        }
        Ok(deleted)
    }
}

// ── PasskeyOps ──

#[async_trait]
impl PasskeyOps for AllsourceAuthAdapter {
    type Passkey = Passkey;

    async fn create_passkey(&self, input: CreatePasskey) -> AuthResult<Passkey> {
        // Check credential_id uniqueness
        if self
            .get_passkey_by_credential_id(&input.credential_id)
            .await?
            .is_some()
        {
            return Err(AuthError::conflict(
                "A passkey with this credential ID already exists",
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let passkey = Passkey {
            id: id.clone(),
            name: input.name,
            public_key: input.public_key,
            user_id: input.user_id,
            credential_id: input.credential_id,
            counter: input.counter,
            device_type: input.device_type,
            backed_up: input.backed_up,
            transports: input.transports,
            created_at: now,
        };

        let payload = serde_json::to_value(&passkey).map_err(AuthError::from)?;
        self.client
            .append_event(&passkey_entity(&id), "auth.passkey.created", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(passkey)
    }

    async fn get_passkey_by_id(&self, id: &str) -> AuthResult<Option<Passkey>> {
        self.client
            .get_latest(&passkey_entity(id))
            .await
            .map_err(AuthError::from)
    }

    async fn get_passkey_by_credential_id(
        &self,
        credential_id: &str,
    ) -> AuthResult<Option<Passkey>> {
        self.client
            .find_by_field("auth.passkey.created", "credentialID", credential_id)
            .await
            .map_err(AuthError::from)
    }

    async fn list_passkeys_by_user(&self, user_id: &str) -> AuthResult<Vec<Passkey>> {
        let mut passkeys: Vec<Passkey> = self
            .client
            .find_all_by_field("auth.passkey.created", "user_id", user_id)
            .await
            .map_err(AuthError::from)?;

        passkeys.sort_by_key(|p| std::cmp::Reverse(p.created_at));
        Ok(passkeys)
    }

    async fn update_passkey_counter(&self, id: &str, counter: u64) -> AuthResult<Passkey> {
        let mut passkey = self
            .get_passkey_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("Passkey not found"))?;

        passkey.counter = counter;

        let payload = serde_json::to_value(&passkey).map_err(AuthError::from)?;
        self.client
            .append_event(&passkey_entity(id), "auth.passkey.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(passkey)
    }

    async fn update_passkey_name(&self, id: &str, name: &str) -> AuthResult<Passkey> {
        let mut passkey = self
            .get_passkey_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("Passkey not found"))?;

        passkey.name = name.to_string();

        let payload = serde_json::to_value(&passkey).map_err(AuthError::from)?;
        self.client
            .append_event(&passkey_entity(id), "auth.passkey.updated", payload)
            .await
            .map_err(AuthError::from)?;

        Ok(passkey)
    }

    async fn delete_passkey(&self, id: &str) -> AuthResult<()> {
        self.client
            .append_delete(&passkey_entity(id), "auth.passkey.deleted")
            .await
            .map_err(AuthError::from)
    }
}
