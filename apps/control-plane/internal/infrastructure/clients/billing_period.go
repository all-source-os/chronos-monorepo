package clients

import "strings"

// Canonical billing periods used across payment-provider clients.
const (
	periodMonthly = "monthly"
	periodAnnual  = "annual"
)

// normalizeBillingPeriod maps a user-supplied billing period to its canonical
// form. Empty or unrecognized values fall back to monthly; "yearly" is an
// accepted input alias for the canonical "annual".
func normalizeBillingPeriod(period string) string {
	switch strings.ToLower(period) {
	case periodAnnual, "yearly":
		return periodAnnual
	default:
		return periodMonthly
	}
}
