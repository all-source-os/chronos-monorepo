package dto

// AdminInvoiceListRequest contains parameters for listing invoices.
type AdminInvoiceListRequest struct {
	TenantID string `json:"tenant_id"`
	Status   string `json:"status"`
	Page     int    `json:"page"`
}

// AdminInvoiceItem represents a single invoice in admin responses.
type AdminInvoiceItem struct {
	ID             string `json:"id"`
	TenantID       string `json:"tenant_id,omitempty"`
	CustomerID     int    `json:"customer_id"`
	SubscriptionID int    `json:"subscription_id,omitempty"`
	Status         string `json:"status"`
	TotalFormatted string `json:"total_formatted"`
	Total          int    `json:"total"`
	Currency       string `json:"currency"`
	CreatedAt      string `json:"created_at"`
	RefundedAt     string `json:"refunded_at,omitempty"`
}

// AdminInvoiceListResponse wraps a paginated list of invoices for admin.
type AdminInvoiceListResponse struct {
	Invoices   []AdminInvoiceItem `json:"invoices"`
	Total      int                `json:"total"`
	Page       int                `json:"page"`
	TotalPages int                `json:"total_pages"`
}

// AdminRevenueRequest contains parameters for revenue metrics.
type AdminRevenueRequest struct {
	Range string `json:"range"` // 30d, 90d, 1y
}

// AdminRevenueResponse contains revenue metrics.
type AdminRevenueResponse struct {
	MRR        float64              `json:"mrr"`
	ARR        float64              `json:"arr"`
	GrowthRate float64              `json:"growth_rate"`
	ChurnRate  float64              `json:"churn_rate"`
	Period     string               `json:"period"`
	Breakdown  []RevenueBreakdown   `json:"breakdown,omitempty"`
	TierStats  map[string]TierStats `json:"tier_stats,omitempty"`
}

// RevenueBreakdown represents revenue for a single period (month).
type RevenueBreakdown struct {
	Period       string  `json:"period"` // YYYY-MM
	Revenue      float64 `json:"revenue"`
	Subscribers  int     `json:"subscribers"`
	NewCustomers int     `json:"new_customers"`
	Churned      int     `json:"churned"`
}

// TierStats holds subscription counts and revenue per tier.
type TierStats struct {
	Count   int     `json:"count"`
	Revenue float64 `json:"revenue"`
}

// AdminRefundRequest contains parameters for processing a refund.
type AdminRefundRequest struct {
	OrderID string `json:"order_id" binding:"required"`
	Amount  int    `json:"amount" binding:"required"` // in cents
	Reason  string `json:"reason,omitempty"`
}

// AdminRefundResponse contains the result of a refund operation.
type AdminRefundResponse struct {
	OrderID  string `json:"order_id"`
	Amount   int    `json:"amount"`
	Status   string `json:"status"`
	RefundID string `json:"refund_id,omitempty"`
}

// AdminDunningItem represents a tenant with failed payment status.
type AdminDunningItem struct {
	TenantID       string `json:"tenant_id"`
	TenantName     string `json:"tenant_name"`
	SubscriptionID string `json:"subscription_id"`
	Status         string `json:"status"` // past_due, unpaid, expired
	Tier           string `json:"tier"`
	CustomerID     string `json:"customer_id"`
	LastPayment    string `json:"last_payment,omitempty"`
	RetryStatus    string `json:"retry_status"` // pending_retry, exhausted, manual_review
}

// AdminDunningResponse wraps a list of tenants with failed payments.
type AdminDunningResponse struct {
	Items      []AdminDunningItem `json:"items"`
	TotalCount int                `json:"total_count"`
}
