package main

import (
	"crypto/rand"
	"strings"
	"testing"
	"unicode/utf8"
)

func TestMaskLicenseKeyKeepsOnlyTheTail(t *testing.T) {
	// "OZ-PRO-XXXX-XXXX-XXXX-DEADBEAF" shape: the identifying part must
	// survive, the rest must not.
	got := maskLicenseKey("OZ-PRO-ABCD-EFGH-JKMN-PQRS")
	want := "...KMN-PQRS"
	if got != want {
		t.Fatalf("maskLicenseKey() = %q, want %q", got, want)
	}
	if strings.Contains(got, "ABCD") || strings.Contains(got, "OZ-PRO") {
		t.Errorf("masked value still carries the prefix: %q", got)
	}
}

func TestMaskLicenseKeyNeverEmitsTheWholeSecret(t *testing.T) {
	// The property that actually matters, checked over a range of lengths
	// rather than one example: the output must never contain the input, and
	// must never be longer than it.
	cases := []struct {
		name string
		in   string
	}{
		{"empty", ""},
		{"one char", "A"},
		{"half of tail", "ABCDEFGH"},
		{"one over half", "ABCDEFGHI"},
		{"exactly twice tail", "ABCDEFGHIJKLMNOP"},
		{"real generated shape", "OZ-STR-ABCD-EFGH-JKMN-PQRS"},
		{"short attacker string", "123456789"},
		{"long attacker string", strings.Repeat("X", 4096)},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := maskLicenseKey(tc.in)
			if tc.in != "" && got == tc.in {
				t.Fatalf("masking returned the secret unchanged: %q", got)
			}
			// Fixed-width output, so the mask is not a length oracle: either
			// the constant "***" for short values, or exactly "..." plus 8
			// runes. Note this replaces a weaker first attempt that asserted
			// the mask is never longer than the secret — "***" is longer than
			// a 1-character input and that is harmless, because it reveals
			// nothing; leaking a suffix would not be.
			if got != "***" {
				if n := len([]rune(got)); n != 11 {
					t.Errorf("expected the fixed 11-rune tail form, got %d runes: %q", n, got)
				}
				if !strings.HasPrefix(got, "...") {
					t.Errorf("unexpected form %q", got)
				}
			}
			if got == "" {
				t.Error("masking returned an empty string; log lines need a placeholder")
			}
		})
	}
}

func TestMaskLicenseKeyMasksShortValuesEntirely(t *testing.T) {
	// At or below twice the tail there is nothing worth showing: "...X" for
	// a 9-char key leaks most of it.
	for _, in := range []string{"", "A", "ABCDEFGH", "ABCDEFGHI", "12345678"} {
		if got := maskLicenseKey(in); got != "***" {
			t.Errorf("maskLicenseKey(%q) = %q, want %q", in, got, "***")
		}
	}
}

func TestMaskLicenseKeyIsRuneSafe(t *testing.T) {
	// Inputs come from request bodies (req.Key, body.Key), so this is
	// attacker-controlled and may be multi-byte. Byte slicing here would
	// split a rune and write invalid UTF-8 into the log stream.
	in := strings.Repeat("é", 20) + "TAILHERE"
	got := maskLicenseKey(in)
	if !utf8.ValidString(got) {
		t.Errorf("masked output is not valid UTF-8: %q", got)
	}
	if got != "...TAILHERE" && got != "***" {
		// The tail is 8 runes; "TAILHERE" is exactly 8 ASCII runes.
		t.Errorf("unexpected result %q", got)
	}

	// A string of only multi-byte runes must not panic and must not echo
	// more than 8 runes.
	onlyMultibyte := strings.Repeat("漢", 30)
	res := maskLicenseKey(onlyMultibyte)
	if n := len([]rune(res)); n > 11 { // "..." + 8 runes
		t.Errorf("masked output too long: %d runes, %q", n, res)
	}
}

func TestMaskLicenseKeyIsDeterministic(t *testing.T) {
	// Logs are grepped and correlated, so the same key must always render
	// the same way. A randomised mask would defeat the point.
	key := "OZ-ENT-ABCD-EFGH-JKMN-PQRS"
	first := maskLicenseKey(key)
	for i := 0; i < 3; i++ {
		if got := maskLicenseKey(key); got != first {
			t.Fatalf("not deterministic: %q vs %q", got, first)
		}
	}
}

func TestMaskLicenseKeyOnRealGeneratedKeys(t *testing.T) {
	// End-to-end against the actual generator, so the assertions hold for
	// the format that really ships rather than a hand-written example.
	seen := make(map[string]string)
	for i := 0; i < 200; i++ {
		key, err := generateLicenseKey("pro")
		if err != nil {
			t.Fatalf("generateLicenseKey: %v", err)
		}
		masked := maskLicenseKey(key)

		if strings.Contains(masked, key) {
			t.Fatalf("masked %q contains the full key", masked)
		}
		if !strings.HasPrefix(masked, "...") {
			t.Errorf("expected a tail form, got %q", masked)
		}
		// The tier must not be recoverable from the mask alone, since the
		// whole point is that the prefix is dropped.
		if strings.Contains(masked, "OZ-") {
			t.Errorf("prefix leaked into %q", masked)
		}
		// Correlation: a distinct mask per key, unless a genuine 7-symbol
		// collision occurred.
		if prev, ok := seen[masked]; ok && prev != key {
			t.Logf("collision (expected occasionally): %s and %s both mask to %s", prev, key, masked)
		}
		seen[masked] = key
	}
	// 32^7 ≈ 3.4e10 buckets, so 200 keys should be essentially all distinct.
	if len(seen) < 190 {
		t.Errorf("too many collisions: %d distinct masks from 200 keys", len(seen))
	}
}

func TestMaskLicenseKeySurvivesRandomBytes(t *testing.T) {
	// Fuzz-ish: arbitrary bytes, including NUL and invalid UTF-8, must not
	// panic. These values reach the mask from parsed request bodies.
	for i := 0; i < 100; i++ {
		buf := make([]byte, 33)
		if _, err := rand.Read(buf); err != nil {
			t.Skipf("no entropy available: %v", err)
		}
		_ = maskLicenseKey(string(buf))
	}
}
