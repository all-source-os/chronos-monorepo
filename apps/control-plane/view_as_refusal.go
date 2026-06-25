package main

import (
	"context"
	"net/http"

	"github.com/gin-gonic/gin"
)

// viewAsAlarmFunc records the durable "view_as token attempted a write" alarm
// (admin.viewas.write_refused). It is satisfied by
// usecases.ViewAsAuditor.RecordWriteRefused — passed as a method value so this
// file does not import the usecases package just for the type. nil disables the
// alarm write (the refusal itself still happens — the 403 is the hard guard).
type viewAsAlarmFunc func(ctx context.Context, tenantID, actor, method, path string) error

// isMutatingMethod reports whether an HTTP method can change state. The view-as
// write-refusal fires on exactly these so a read-only impersonation token can
// never reach a mutating handler.
func isMutatingMethod(method string) bool {
	switch method {
	case http.MethodPost, http.MethodPut, http.MethodPatch, http.MethodDelete:
		return true
	default:
		return false
	}
}

// ViewAsWriteRefusal is the defense-in-depth middleware that hard-refuses a
// view_as:true token on ANY mutating request (POST/PUT/PATCH/DELETE), independent
// of the role (ADMIN_TENANT_POWER_TOOL §5.2 layer 2). It is registered in the
// global chain DIRECTLY AFTER AuthMiddleware so it covers the entire data-plane
// surface a view-as token could be presented to, and it reads the validated
// claims AuthMiddleware stashed under "auth_claims" (no re-parse).
//
// Why belt-and-suspenders: the readonly role already blocks writes (RoleReadOnly
// has no Write permission), but the cost of a single write on a customer's behalf
// is unacceptable, so a view_as token is refused on writes EVEN IF a write route
// ever accidentally accepted readonly. Every refusal is alarmed as a durable Core
// event — a write ATTEMPT is itself the security signal (there is no
// admin.viewas.wrote event because view-as never writes).
func ViewAsWriteRefusal(alarm viewAsAlarmFunc) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Only mutating methods can do harm — reads with a view_as token are the
		// entire point of the feature, so they pass straight through.
		if !isMutatingMethod(c.Request.Method) {
			c.Next()
			return
		}

		claimsVal, ok := c.Get("auth_claims")
		if !ok {
			// No validated local-JWT claims on this request (public path, ask_ key,
			// or admin route which uses its own middleware). Nothing to refuse here.
			c.Next()
			return
		}
		claims, ok := claimsVal.(*Claims)
		if !ok || claims == nil || !claims.ViewAs {
			c.Next()
			return
		}

		// A view_as token reached a mutating route. REFUSE — read-only by
		// construction. Record the alarm (best-effort; the 403 is the hard guard).
		actor := claims.ActAs
		if actor == "" {
			actor = claims.UserID
		}
		if alarm != nil {
			// Best-effort durable alarm; do not block the refusal on the audit write.
			_ = alarm(c.Request.Context(), claims.TenantID, actor, c.Request.Method, c.Request.URL.Path) //nolint:errcheck // refusal below is the hard guard; alarm is the durable record
		}

		c.JSON(http.StatusForbidden, gin.H{
			"error":   "forbidden",
			"message": "view-as is read-only: write refused (view_as token cannot mutate)",
			"view_as": true,
		})
		c.Abort()
	}
}
