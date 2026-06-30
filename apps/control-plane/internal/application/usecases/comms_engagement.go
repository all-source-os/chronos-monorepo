package usecases

import (
	"context"
	"strings"
)

// Engagement ingress (prompt 050). The Resend webhook handler (interfaces/http)
// stays thin: it verifies the Svix signature, parses the delivery, and calls
// RecordEngagement. ALL schema knowledge — type mapping, message_id → tag
// resolution, idempotent emission — lives HERE so the engagement shape has a
// single owner and the in-flight Resend handler change is minimal (one method call).

// Engagement outcome statuses RecordEngagement returns (the handler maps them to
// HTTP: ingested/duplicate → 200, ignored → 200 ignored).
const (
	EngagementIngested  = "ingested"
	EngagementDuplicate = "duplicate"
	EngagementIgnored   = "ignored"
)

// mapResendEngagementType maps a Resend webhook event type to our engagement Core
// event type. Returns ok=false for types we don't fold into the funnel
// (email.sent — already recorded as admin.message.sent at send time; and
// delivery_delayed / non-email types). The mapping is an explicit allowlist so an
// unknown future type is ignored, never mis-ingested.
func mapResendEngagementType(espType string) (string, bool) {
	switch strings.TrimSpace(espType) {
	case "email.delivered":
		return EmailDeliveredEventType, true
	case "email.opened":
		return EmailOpenedEventType, true
	case "email.clicked":
		return EmailClickedEventType, true
	case "email.bounced":
		return EmailBouncedEventType, true
	case "email.complained":
		return EmailComplainedEventType, true
	case "email.unsubscribed":
		// Resend's email.* family has no native unsubscribe; included so a
		// List-Unsubscribe handler or another ESP can feed the same funnel.
		return EmailUnsubscribedEventType, true
	default:
		return "", false
	}
}

// RecordEngagement maps an ESP engagement webhook to a durable engagement Core
// event, resolving the ESP message id back to the send's correlation tags
// (tenant, campaign, variant, …) and emitting idempotently. A replayed webhook for
// the same (message_id, type) is a no-op (returns EngagementDuplicate) — the funnel
// is never double-counted.
//
// When the correlation record is absent (e.g. a send made before instrumentation,
// or a non-comms ESP message), the event is STILL recorded keyed by message_id so
// no signal is lost — it simply lands in an untagged group the operator can spot.
func (uc *CommsUseCase) RecordEngagement(ctx context.Context, espMessageID, espEventType, eventTS, link string) (string, error) {
	ourType, ok := mapResendEngagementType(espEventType)
	if !ok {
		return EngagementIgnored, nil
	}
	espMessageID = strings.TrimSpace(espMessageID)
	if espMessageID == "" {
		return EngagementIgnored, nil
	}

	// Resolve message_id → (tenant, campaign, variant, …). Missing is non-fatal.
	tags, _ := uc.recorder.getCorrelation(ctx, espMessageID)
	tags.MessageID = espMessageID

	payload := map[string]any{}
	tags.ApplyTo(payload)
	if eventTS != "" {
		payload[TagEventTS] = eventTS
	}
	if link != "" {
		payload[TagLink] = link
	}

	duplicate, err := uc.recorder.recordIdempotent(ctx, ourType, commsEngagementEntityID(espMessageID, ourType), payload)
	if err != nil {
		return "", err
	}
	if duplicate {
		return EngagementDuplicate, nil
	}
	return EngagementIngested, nil
}
