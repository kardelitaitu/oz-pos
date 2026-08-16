package main

// Shared SMTP delivery for the OTP and receipt senders (web_otp.go and
// paddle_webhook.go both call sendMailSMTP). Config is read per call by
// the callers so tests can t.Setenv and ops can fix a relay with a
// redeploy; credentials are never echoed in responses or logs.
//
// Transport selection by port:
//
//   - 465  → implicit TLS: the connection is TLS-encrypted from the very
//     first byte (Brevo Option B, plus common relays such as SendGrid's
//     465 endpoint). Go's smtp.SendMail cannot do this (it only supports
//     STARTTLS), so this path dials TLS itself and drives the SMTP
//     conversation manually.
//   - anything else (587, 2525, 25) → smtp.SendMail, i.e. a plaintext
//     connection upgraded with STARTTLS when the server advertises it
//     (Brevo Options A/C).
//
// Auth is always PLAIN over the encrypted channel (implicit TLS, or
// STARTTLS negotiated by SendMail). OZ_SMTP_USER empty → unauthenticated.

import (
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"net"
	"net/smtp"
)

// sendMailSMTP delivers msg to the recipients via host:port using the
// given credentials, choosing the transport by port (see file comment).
func sendMailSMTP(host, port, user, password, from string, to []string, msg []byte) error {
	addr := net.JoinHostPort(host, port)
	var auth smtp.Auth
	if user != "" {
		auth = smtp.PlainAuth("", user, password, host)
	}
	if port == "465" {
		return sendMailImplicitTLS(addr, host, auth, from, to, msg)
	}
	if err := smtp.SendMail(addr, auth, from, to, msg); err != nil {
		return fmt.Errorf("smtp.SendMail: %w", err)
	}
	return nil
}

// smtpTLSRootCAs is a package-level test hook: nil uses the system root
// pool (production — real relays have publicly trusted certs). Tests point
// it at the in-process server's self-signed cert so the implicit-TLS dial
// verifies without weakening the production path.
var smtpTLSRootCAs *x509.CertPool

// sendMailImplicitTLS delivers msg over a connection that is TLS-encrypted
// from the first byte (port 465). This mirrors what smtp.SendMail does for
// the STARTTLS case, minus the unencrypted preamble.
func sendMailImplicitTLS(addr, host string, auth smtp.Auth, from string, to []string, msg []byte) error {
	cfg := &tls.Config{ServerName: host, MinVersion: tls.VersionTLS12}
	if smtpTLSRootCAs != nil {
		cfg.RootCAs = smtpTLSRootCAs
	}
	conn, err := tls.Dial("tcp", addr, cfg)
	if err != nil {
		return fmt.Errorf("tls.Dial: %w", err)
	}
	c, err := smtp.NewClient(conn, host)
	if err != nil {
		conn.Close()
		return fmt.Errorf("smtp.NewClient: %w", err)
	}
	defer c.Close()

	if auth != nil {
		if ok, _ := c.Extension("AUTH"); ok {
			if err := c.Auth(auth); err != nil {
				return fmt.Errorf("smtp.Auth: %w", err)
			}
		}
	}
	if err := c.Mail(from); err != nil {
		return fmt.Errorf("smtp.Mail: %w", err)
	}
	for _, rcpt := range to {
		if err := c.Rcpt(rcpt); err != nil {
			return fmt.Errorf("smtp.Rcpt(%s): %w", rcpt, err)
		}
	}
	w, err := c.Data()
	if err != nil {
		return fmt.Errorf("smtp.Data: %w", err)
	}
	if _, err := w.Write(msg); err != nil {
		return fmt.Errorf("smtp data write: %w", err)
	}
	if err := w.Close(); err != nil {
		return fmt.Errorf("smtp data close: %w", err)
	}
	if err := c.Quit(); err != nil {
		return fmt.Errorf("smtp.Quit: %w", err)
	}
	return nil
}
