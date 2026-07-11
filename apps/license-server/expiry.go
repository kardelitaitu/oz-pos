package main

import "time"

// calculateExpiry returns the subscription expiration time for the given tier.
//
// Tier durations:
//   - free:      lifetime (100 years from now)
//   - pro:       1 year
//   - premium:   1 year
//   - enterprise: 3 years (configurable per contract)
func calculateExpiry(tier string) time.Time {
	now := time.Now().UTC()
	switch tier {
	case "free":
		return now.AddDate(100, 0, 0) // effectively never expires
	case "pro", "premium":
		return now.AddDate(1, 0, 0)
	case "enterprise":
		return now.AddDate(3, 0, 0)
	default:
		return now.AddDate(1, 0, 0) // safe default: 1 year
	}
}

// calculateGraceUntil returns expires_at + 14 days (per ADR #5 offline grace).
func calculateGraceUntil(expiresAt time.Time) time.Time {
	return expiresAt.AddDate(0, 0, 14)
}
