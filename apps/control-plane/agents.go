package main

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
)

// AgentRegisterHandler handles POST /api/v1/agents/register.
// Thin HTTP adapter — all business logic lives in RegisterAgentUseCase.
func (cp *ControlPlane) AgentRegisterHandler(c *gin.Context) {
	var req dto.RegisterAgentRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{
			"error":   "invalid_request",
			"message": err.Error(),
		})
		return
	}

	resp, err := cp.container.RegisterAgentUC.Execute(c.Request.Context(), req)
	if err != nil {
		if errors.Is(err, domain.ErrTenantAlreadyExists) {
			c.JSON(http.StatusConflict, gin.H{
				"error":   "agent_exists",
				"message": "An agent with this name is already registered.",
			})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{
			"error":   "registration_failed",
			"message": err.Error(),
		})
		return
	}

	c.JSON(http.StatusCreated, resp)
}
