package http //nolint:revive // package name intentionally matches directory

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// BillingConfigHandler exposes the billing config self-check as an admin
// diagnostic so config drift (variant-map gaps, bad webhook secret) can be
// inspected on demand without redeploying or reading boot logs.
type BillingConfigHandler struct {
	verifyUC *usecases.VerifyBillingConfigUseCase
}

// NewBillingConfigHandler constructs the handler.
func NewBillingConfigHandler(verifyUC *usecases.VerifyBillingConfigUseCase) *BillingConfigHandler {
	return &BillingConfigHandler{verifyUC: verifyUC}
}

// ConfigCheck runs the verification and returns the structured report. Always
// 200 (it's a diagnostic, not a probe); callers gate on report.ok.
func (h *BillingConfigHandler) ConfigCheck(c *gin.Context) {
	c.JSON(http.StatusOK, h.verifyUC.Execute())
}
