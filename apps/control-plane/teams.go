package main

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// AgentKeyMeta is the persisted metadata for a provisioned agent API key.
// The actual key value is returned once at creation and never stored.
type AgentKeyMeta struct {
	Name      string `json:"name"`
	KeyID     string `json:"key_id"`
	CreatedAt string `json:"created_at"`
}

// TeamMember represents a member of a team (tenant).
type TeamMember struct {
	UserID   string `json:"user_id"`
	Email    string `json:"email"`
	Name     string `json:"name"`
	Role     string `json:"role"`
	JoinedAt string `json:"joined_at"`
}

// TeamInvite represents a pending team invite.
type TeamInvite struct {
	Token     string `json:"token"`
	TenantID  string `json:"tenant_id"`
	Email     string `json:"email"`
	Role      string `json:"role"`
	InvitedBy string `json:"invited_by"`
	CreatedAt string `json:"created_at"`
}

// InviteRequest is the request body for creating an invite.
type InviteRequest struct {
	Email string `json:"email" binding:"required"`
	Role  string `json:"role"` // "admin" or "member" (default: "member")
}

const (
	roleAdmin  = "admin"
	roleMember = "member"
)

// teamInviteConfigKey returns the Core config key for an invite token.
func teamInviteConfigKey(token string) string {
	return "team:invite:" + token
}

// teamMembersConfigKey returns the Core config key for a tenant's member list.
func teamMembersConfigKey(tenantID string) string {
	return "team:" + tenantID + ":members"
}

// generateInviteToken generates a random URL-safe token.
func generateInviteToken() (string, error) {
	b := make([]byte, 24)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(b), nil
}

// InviteHandler creates a team invite.
// POST /api/v1/teams/invite
// Requires: admin role
func (cp *ControlPlane) InviteHandler(c *gin.Context) {
	authCtx := mustAuthContext(c)
	if authCtx == nil {
		return
	}
	if authCtx.Role != roleAdmin {
		c.JSON(403, gin.H{"error": "forbidden", "message": "admin role required to invite team members"})
		return
	}

	var req InviteRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}

	role := req.Role
	if role == "" {
		role = roleMember
	}
	if role != roleAdmin && role != roleMember {
		c.JSON(400, gin.H{"error": "invalid role", "message": "role must be 'admin' or 'member'"})
		return
	}

	token, err := generateInviteToken()
	if err != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to generate invite token"})
		return
	}

	invite := TeamInvite{
		Token:     token,
		TenantID:  authCtx.TenantID,
		Email:     req.Email,
		Role:      role,
		InvitedBy: authCtx.UserID,
		CreatedAt: time.Now().UTC().Format(time.RFC3339),
	}

	if storeErr := cp.coreClient.SetConfig(c.Request.Context(), clients.SetConfigRequest{
		Key:       teamInviteConfigKey(token),
		Value:     invite,
		ChangedBy: authCtx.UserID,
	}); storeErr != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to store invite"})
		return
	}

	c.JSON(201, gin.H{
		"token":      token,
		"email":      req.Email,
		"role":       role,
		"created_at": invite.CreatedAt,
	})
}

// GetInviteHandler returns invite details for the given token.
// GET /api/v1/teams/invite/:token
// Public — used by the frontend to show the user what they're joining before OAuth.
func (cp *ControlPlane) GetInviteHandler(c *gin.Context) {
	token := c.Param("token")
	if token == "" {
		c.JSON(400, gin.H{"error": "missing token"})
		return
	}

	entry, err := cp.coreClient.GetConfig(c.Request.Context(), teamInviteConfigKey(token))
	if err != nil {
		c.JSON(404, gin.H{"error": "not_found", "message": "invite not found or expired"})
		return
	}

	invite, parseErr := parseInviteFromConfig(entry.Value)
	if parseErr != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "malformed invite record"})
		return
	}

	c.JSON(200, gin.H{
		"token":      invite.Token,
		"email":      invite.Email,
		"role":       invite.Role,
		"tenant_id":  invite.TenantID,
		"invited_by": invite.InvitedBy,
		"created_at": invite.CreatedAt,
	})
}

// ListMembersHandler lists all members of the caller's team.
// GET /api/v1/teams/members
func (cp *ControlPlane) ListMembersHandler(c *gin.Context) {
	authCtx := mustAuthContext(c)
	if authCtx == nil {
		return
	}

	members, err := cp.getTeamMembers(c.Request.Context(), authCtx.TenantID)
	if err != nil {
		// No members stored yet is not an error — return empty list.
		members = []TeamMember{}
	}

	c.JSON(200, gin.H{"members": members})
}

// DeleteMemberHandler removes a member from the team.
// DELETE /api/v1/teams/members/:id
// Requires: admin role
func (cp *ControlPlane) DeleteMemberHandler(c *gin.Context) {
	authCtx := mustAuthContext(c)
	if authCtx == nil {
		return
	}
	if authCtx.Role != roleAdmin {
		c.JSON(403, gin.H{"error": "forbidden", "message": "admin role required to remove team members"})
		return
	}

	memberID := c.Param("id")
	if memberID == "" {
		c.JSON(400, gin.H{"error": "missing member id"})
		return
	}
	if memberID == authCtx.UserID {
		c.JSON(400, gin.H{"error": "invalid_operation", "message": "cannot remove yourself from the team"})
		return
	}

	members, err := cp.getTeamMembers(c.Request.Context(), authCtx.TenantID)
	if err != nil {
		c.JSON(404, gin.H{"error": "not_found", "message": "member not found"})
		return
	}

	updated := make([]TeamMember, 0, len(members))
	found := false
	for _, m := range members {
		if m.UserID == memberID {
			found = true
			continue
		}
		updated = append(updated, m)
	}

	if !found {
		c.JSON(404, gin.H{"error": "not_found", "message": "member not found"})
		return
	}

	if saveErr := cp.saveTeamMembers(c.Request.Context(), authCtx.TenantID, updated, authCtx.UserID); saveErr != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to update members"})
		return
	}

	c.Status(204)
}

// UpdateMemberRoleHandler changes the role of a team member.
// PUT /api/v1/teams/members/:id
// Requires: admin role
func (cp *ControlPlane) UpdateMemberRoleHandler(c *gin.Context) {
	authCtx := mustAuthContext(c)
	if authCtx == nil {
		return
	}
	if authCtx.Role != roleAdmin {
		c.JSON(403, gin.H{"error": "forbidden", "message": "admin role required to change member roles"})
		return
	}

	memberID := c.Param("id")
	if memberID == "" {
		c.JSON(400, gin.H{"error": "missing member id"})
		return
	}

	var req struct {
		Role string `json:"role" binding:"required"`
	}
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}
	if req.Role != roleAdmin && req.Role != "member" {
		c.JSON(400, gin.H{"error": "invalid role", "message": "role must be 'admin' or 'member'"})
		return
	}

	members, err := cp.getTeamMembers(c.Request.Context(), authCtx.TenantID)
	if err != nil {
		c.JSON(404, gin.H{"error": "not_found", "message": "member not found"})
		return
	}

	found := false
	for i, m := range members {
		if m.UserID == memberID {
			members[i].Role = req.Role
			found = true
			break
		}
	}

	if !found {
		c.JSON(404, gin.H{"error": "not_found", "message": "member not found"})
		return
	}

	if saveErr := cp.saveTeamMembers(c.Request.Context(), authCtx.TenantID, members, authCtx.UserID); saveErr != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to update member role"})
		return
	}

	c.JSON(200, gin.H{"message": "role updated"})
}

// AddTeamMember appends a member to the tenant's member list in Core config.
// Called by the OAuth invite-accept flow (US-004).
func (cp *ControlPlane) AddTeamMember(ctx context.Context, tenantID string, member TeamMember, callerID string) error {
	members, err := cp.getTeamMembers(ctx, tenantID)
	if err != nil {
		members = []TeamMember{}
	}

	// Deduplicate: skip if already a member.
	for _, m := range members {
		if m.UserID == member.UserID {
			return nil
		}
	}

	members = append(members, member)
	return cp.saveTeamMembers(ctx, tenantID, members, callerID)
}

// getTeamMembers fetches the member list from Core config.
func (cp *ControlPlane) getTeamMembers(ctx context.Context, tenantID string) ([]TeamMember, error) {
	entry, err := cp.coreClient.GetConfig(ctx, teamMembersConfigKey(tenantID))
	if err != nil {
		return nil, err
	}
	if entry == nil {
		return []TeamMember{}, nil
	}
	return parseMembersFromConfig(entry.Value)
}

// saveTeamMembers persists the member list to Core config.
func (cp *ControlPlane) saveTeamMembers(ctx context.Context, tenantID string, members []TeamMember, callerID string) error {
	return cp.coreClient.SetConfig(ctx, clients.SetConfigRequest{
		Key:       teamMembersConfigKey(tenantID),
		Value:     members,
		ChangedBy: callerID,
	})
}

// parseInviteFromConfig converts the raw config value into a TeamInvite.
func parseInviteFromConfig(raw any) (*TeamInvite, error) {
	b, err := json.Marshal(raw)
	if err != nil {
		return nil, fmt.Errorf("marshal invite: %w", err)
	}
	var invite TeamInvite
	if err := json.Unmarshal(b, &invite); err != nil {
		return nil, fmt.Errorf("unmarshal invite: %w", err)
	}
	return &invite, nil
}

// parseMembersFromConfig converts the raw config value into []TeamMember.
func parseMembersFromConfig(raw any) ([]TeamMember, error) {
	b, err := json.Marshal(raw)
	if err != nil {
		return nil, fmt.Errorf("marshal members: %w", err)
	}
	var members []TeamMember
	if err := json.Unmarshal(b, &members); err != nil {
		return nil, fmt.Errorf("unmarshal members: %w", err)
	}
	return members, nil
}

// resolveInvite looks up a pending invite by token and validates it for the given email.
// Returns the invite if it exists and either has no email restriction or matches the given email.
func (cp *ControlPlane) resolveInvite(ctx context.Context, token, email string) (*TeamInvite, error) {
	if token == "" {
		return nil, fmt.Errorf("no token")
	}
	entry, err := cp.coreClient.GetConfig(ctx, teamInviteConfigKey(token))
	if err != nil || entry == nil {
		return nil, fmt.Errorf("invite not found")
	}
	invite, err := parseInviteFromConfig(entry.Value)
	if err != nil {
		return nil, err
	}
	if invite.Email != "" && invite.Email != email {
		return nil, fmt.Errorf("invite email mismatch")
	}
	return invite, nil
}

// teamAgentKeysConfigKey returns the Core config key for a tenant's agent key list.
func teamAgentKeysConfigKey(tenantID string) string {
	return "team:" + tenantID + ":agent_keys"
}

// CreateAgentKeyHandler provisions a new Core API key for an agent working in the team's tenant.
// POST /api/v1/teams/agent-keys
// The raw key value is returned exactly once — it is never stored or retrievable again.
func (cp *ControlPlane) CreateAgentKeyHandler(c *gin.Context) {
	authCtx := mustAuthContext(c)
	if authCtx == nil {
		return
	}

	var req struct {
		Name string `json:"name" binding:"required"`
	}
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid_request", "message": err.Error()})
		return
	}

	// Create a ServiceAccount key scoped to the team tenant in Core.
	resp, err := cp.coreClient.CreateCoreAPIKey(c.Request.Context(), clients.CreateCoreAPIKeyRequest{
		Name:     req.Name,
		TenantID: authCtx.TenantID,
		Role:     "serviceaccount",
	})
	if err != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to create agent key"})
		return
	}

	// Persist metadata (name + Core key ID) so the key can be listed and revoked later.
	// The raw key value is NOT stored — it is only returned in this response.
	createdAt := time.Now().UTC().Format(time.RFC3339)
	meta := AgentKeyMeta{Name: req.Name, KeyID: resp.ID, CreatedAt: createdAt}

	existing, _ := cp.getAgentKeyMetas(c.Request.Context(), authCtx.TenantID) //nolint:errcheck
	existing = append(existing, meta)
	if saveErr := cp.saveAgentKeyMetas(c.Request.Context(), authCtx.TenantID, existing, authCtx.UserID); saveErr != nil {
		// Non-fatal: the key was created in Core; warn but still return it.
		c.Header("X-Warning", "key created but metadata save failed; note the key now")
	}

	c.JSON(201, gin.H{
		"name":       req.Name,
		"key":        resp.Key,
		"tenant_id":  authCtx.TenantID,
		"created_at": createdAt,
	})
}

// ListAgentKeysHandler returns metadata for all agent keys provisioned for the team.
// GET /api/v1/teams/agent-keys
// Key values are never returned — only name, key_id, and created_at.
func (cp *ControlPlane) ListAgentKeysHandler(c *gin.Context) {
	authCtx := mustAuthContext(c)
	if authCtx == nil {
		return
	}

	metas, err := cp.getAgentKeyMetas(c.Request.Context(), authCtx.TenantID)
	if err != nil {
		metas = []AgentKeyMeta{}
	}

	c.JSON(200, gin.H{"agent_keys": metas})
}

// RevokeAgentKeyHandler revokes an agent key by name, removing it from Core and the metadata list.
// DELETE /api/v1/teams/agent-keys/:name
func (cp *ControlPlane) RevokeAgentKeyHandler(c *gin.Context) {
	authCtx := mustAuthContext(c)
	if authCtx == nil {
		return
	}

	name := c.Param("name")
	if name == "" {
		c.JSON(400, gin.H{"error": "missing key name"})
		return
	}

	metas, err := cp.getAgentKeyMetas(c.Request.Context(), authCtx.TenantID)
	if err != nil {
		c.JSON(404, gin.H{"error": "not_found", "message": "no agent keys found"})
		return
	}

	var keyID string
	updated := make([]AgentKeyMeta, 0, len(metas))
	for _, m := range metas {
		if m.Name == name {
			keyID = m.KeyID
			continue
		}
		updated = append(updated, m)
	}

	if keyID == "" {
		c.JSON(404, gin.H{"error": "not_found", "message": "agent key not found"})
		return
	}

	// Revoke in Core — this makes the key immediately invalid.
	if revokeErr := cp.coreClient.RevokeAPIKey(c.Request.Context(), keyID); revokeErr != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to revoke key in Core"})
		return
	}

	// Remove from metadata list.
	if saveErr := cp.saveAgentKeyMetas(c.Request.Context(), authCtx.TenantID, updated, authCtx.UserID); saveErr != nil {
		// Key is already revoked in Core; metadata cleanup failure is non-fatal.
		c.Header("X-Warning", "key revoked but metadata cleanup failed")
	}

	c.Status(204)
}

// getAgentKeyMetas fetches the agent key metadata list from Core config.
func (cp *ControlPlane) getAgentKeyMetas(ctx context.Context, tenantID string) ([]AgentKeyMeta, error) {
	entry, err := cp.coreClient.GetConfig(ctx, teamAgentKeysConfigKey(tenantID))
	if err != nil || entry == nil {
		return nil, fmt.Errorf("no agent keys stored")
	}
	return parseAgentKeyMetasFromConfig(entry.Value)
}

// saveAgentKeyMetas persists the agent key metadata list to Core config.
func (cp *ControlPlane) saveAgentKeyMetas(ctx context.Context, tenantID string, metas []AgentKeyMeta, callerID string) error {
	return cp.coreClient.SetConfig(ctx, clients.SetConfigRequest{
		Key:       teamAgentKeysConfigKey(tenantID),
		Value:     metas,
		ChangedBy: callerID,
	})
}

// parseAgentKeyMetasFromConfig converts a raw config value into []AgentKeyMeta.
func parseAgentKeyMetasFromConfig(raw any) ([]AgentKeyMeta, error) {
	b, err := json.Marshal(raw)
	if err != nil {
		return nil, fmt.Errorf("marshal agent key metas: %w", err)
	}
	var metas []AgentKeyMeta
	if err := json.Unmarshal(b, &metas); err != nil {
		return nil, fmt.Errorf("unmarshal agent key metas: %w", err)
	}
	return metas, nil
}

// mustAuthContext extracts the auth context from gin or responds 401 and returns nil.
func mustAuthContext(c *gin.Context) *AuthContext {
	authCtx, exists := c.Get("auth")
	if !exists {
		c.JSON(401, gin.H{"error": "unauthorized"})
		return nil
	}
	ac, ok := authCtx.(*AuthContext)
	if !ok {
		c.JSON(401, gin.H{"error": "unauthorized"})
		return nil
	}
	return ac
}
