package main

// Tests for the shared SMTP sender (smtp_mail.go): the plaintext/STARTTLS
// path (587/2525) and the implicit-TLS path (465). Each test spins up a
// minimal in-process SMTP server and asserts the message bytes that
// arrive, so the transport selection and the TLS handshake are both
// exercised without any external relay.

import (
	"bufio"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/base64"
	"encoding/pem"
	"math/big"
	"net"
	"net/smtp"
	"net/textproto"
	"strings"
	"testing"
	"time"
)

// smtpCapture records the state of one message received by the in-process
// test server.
type smtpCapture struct {
	from    string
	rcpt    []string
	authUsr string
	data    []byte
}

// runSMTPServer starts a minimal SMTP server on 127.0.0.1:0 and returns
// its address plus a channel that receives one capture per message. The
// server advertises AUTH PLAIN/LOGIN but NOT STARTTLS, so the
// smtp.SendMail path stays on the plaintext connection (it would
// otherwise try to upgrade to TLS and fail). With rejectMailFrom true it
// answers MAIL FROM with a permanent 550 (what relays do for an
// unverified sender identity).
func runSMTPServer(t *testing.T, tlsCfg *tls.Config, rejectMailFrom bool) (addr string, captures chan smtpCapture) {
	t.Helper()
	captures = make(chan smtpCapture, 4)

	var ln net.Listener
	var err error
	if tlsCfg != nil {
		ln, err = tls.Listen("tcp", "127.0.0.1:0", tlsCfg)
	} else {
		ln, err = net.Listen("tcp", "127.0.0.1:0")
	}
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	t.Cleanup(func() { ln.Close() })

	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				return
			}
			go serveSMTPConn(conn, captures, rejectMailFrom)
		}
	}()
	return ln.Addr().String(), captures
}

// serveSMTPConn speaks just enough SMTP for Go's smtp.Client: greeting,
// EHLO with AUTH, AUTH PLAIN, MAIL/RCPT/DATA, QUIT.
func serveSMTPConn(conn net.Conn, captures chan smtpCapture, rejectMailFrom bool) {
	defer conn.Close()
	r := textproto.NewReader(bufio.NewReader(conn))
	w := textproto.NewWriter(bufio.NewWriter(conn))

	write := func(line string) error { return w.PrintfLine("%s", line) }
	if err := write("220 localhost ESMTP test"); err != nil {
		return
	}
	var cap smtpCapture
	for {
		line, err := r.ReadLine()
		if err != nil {
			return
		}
		upper := strings.ToUpper(line)
		switch {
		case strings.HasPrefix(upper, "EHLO"):
			// Multi-line: every line but the last carries a dash.
			write("250-localhost")
			write("250-AUTH PLAIN LOGIN")
			write("250 OK")
		case strings.HasPrefix(upper, "AUTH PLAIN"):
			// base64("\x00user\x00pass")
			raw, err := base64.StdEncoding.DecodeString(strings.TrimSpace(line[len("AUTH PLAIN"):]))
			if err == nil {
				parts := strings.SplitN(string(raw), "\x00", 3)
				if len(parts) == 3 {
					cap.authUsr = parts[1]
				}
			}
			write("235 ok")
		case strings.HasPrefix(upper, "MAIL FROM:"):
			cap.from = strings.Trim(strings.TrimPrefix(line, "MAIL FROM:"), "<>")
			if rejectMailFrom {
				write("550 5.7.1 Sender address is not verified")
			} else {
				write("250 ok")
			}
		case strings.HasPrefix(upper, "RCPT TO:"):
			cap.rcpt = append(cap.rcpt, strings.Trim(strings.TrimPrefix(line, "RCPT TO:"), "<>"))
			write("250 ok")
		case strings.HasPrefix(upper, "DATA"):
			write("354 go ahead")
			var body strings.Builder
			for {
				dline, err := r.ReadLine()
				if err != nil {
					return
				}
				if dline == "." {
					break
				}
				body.WriteString(dline)
				body.WriteString("\r\n")
			}
			cap.data = []byte(body.String())
			write("250 ok")
		case strings.HasPrefix(upper, "QUIT"):
			write("221 bye")
			captures <- cap
			return
		default:
			write("250 ok")
		}
	}
}

// testTLSCert builds a self-signed certificate valid for 127.0.0.1 so the
// production tls.Dial (which verifies ServerName) succeeds against it.
func testTLSCert(t *testing.T) tls.Certificate {
	t.Helper()
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatalf("generate key: %v", err)
	}
	tmpl := x509.Certificate{
		SerialNumber:          big.NewInt(1),
		Subject:               pkix.Name{CommonName: "127.0.0.1"},
		NotBefore:             time.Now().Add(-time.Hour),
		NotAfter:              time.Now().Add(time.Hour),
		IsCA:                  true,
		BasicConstraintsValid: true,
		KeyUsage:              x509.KeyUsageDigitalSignature | x509.KeyUsageCertSign,
		ExtKeyUsage:           []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		IPAddresses:           []net.IP{net.ParseIP("127.0.0.1")},
	}
	der, err := x509.CreateCertificate(rand.Reader, &tmpl, &tmpl, &key.PublicKey, key)
	if err != nil {
		t.Fatalf("create cert: %v", err)
	}
	keyDER, err := x509.MarshalECPrivateKey(key)
	if err != nil {
		t.Fatalf("marshal key: %v", err)
	}
	certPEM := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: der})
	keyPEM := pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: keyDER})
	cert, err := tls.X509KeyPair(certPEM, keyPEM)
	if err != nil {
		t.Fatalf("load keypair: %v", err)
	}
	return cert
}

// TestSendMailSMTP_PlaintextPort exercises the non-465 branch (what
// smtp.SendMail handles): delivery on a plaintext listener, unauthenticated.
func TestSendMailSMTP_PlaintextPort(t *testing.T) {
	addr, captures := runSMTPServer(t, nil, false)
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}

	msg := []byte("From: sender@example.com\r\nSubject: test\r\n\r\nhello\r\n")
	if err := sendMailSMTP(host, port, "", "", "sender@example.com", []string{"rcpt@example.com"}, msg); err != nil {
		t.Fatalf("sendMailSMTP(plaintext): %v", err)
	}

	select {
	case cap := <-captures:
		if cap.from != "sender@example.com" {
			t.Errorf("from = %q, want sender@example.com", cap.from)
		}
		if len(cap.rcpt) != 1 || cap.rcpt[0] != "rcpt@example.com" {
			t.Errorf("rcpt = %v, want [rcpt@example.com]", cap.rcpt)
		}
		if string(cap.data) != string(msg) {
			t.Errorf("data = %q, want %q", cap.data, msg)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("timed out waiting for message")
	}
}

// TestSendMailImplicitTLS exercises the 465 transport directly (the
// public-function routing to it is covered by
// TestSendMailSMTP_SelectsImplicitTLSForPort465): delivery + AUTH PLAIN
// over a TLS-from-first-byte connection.
func TestSendMailImplicitTLS(t *testing.T) {
	cert := testTLSCert(t)
	tlsCfg := &tls.Config{Certificates: []tls.Certificate{cert}}
	addr, captures := runSMTPServer(t, tlsCfg, false)
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}

	// Trust the in-process server's self-signed cert (production path
	// keeps the system root pool via smtpTLSRootCAs == nil).
	leaf, err := x509.ParseCertificate(cert.Certificate[0])
	if err != nil {
		t.Fatalf("parse cert: %v", err)
	}
	pool := x509.NewCertPool()
	pool.AddCert(leaf)
	orig := smtpTLSRootCAs
	smtpTLSRootCAs = pool
	defer func() { smtpTLSRootCAs = orig }()

	auth := smtp.PlainAuth("", "otp-user", "otp-pass", host)
	msg := []byte("From: sender@example.com\r\nSubject: tls test\r\n\r\ntls body\r\n")
	if err := sendMailImplicitTLS(addr, host, auth, "sender@example.com", []string{"rcpt@example.com"}, msg); err != nil {
		t.Fatalf("sendMailImplicitTLS: %v", err)
	}

	select {
	case cap := <-captures:
		if cap.authUsr != "otp-user" {
			t.Errorf("auth user = %q, want otp-user", cap.authUsr)
		}
		if string(cap.data) != string(msg) {
			t.Errorf("data = %q, want %q", cap.data, msg)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("timed out waiting for TLS message")
	}
}

// TestSendMailSMTP_SelectsImplicitTLSForPort465 proves the routing branch
// in sendMailSMTP: port "465" must dial TLS, not smtp.SendMail. A plain
// smtp.SendMail to the same plaintext listener would succeed, so success
// here would mean the branch was NOT taken — we assert the TLS dial fails
// against a non-TLS listener instead.
func TestSendMailSMTP_SelectsImplicitTLSForPort465(t *testing.T) {
	addr, captures := runSMTPServer(t, nil, false)
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}
	_ = captures

	err = sendMailSMTP(host, "465", "", "", "sender@example.com", []string{"rcpt@example.com"}, []byte("x"))
	if err == nil {
		t.Fatalf("sendMailSMTP(port 465) against a plaintext listener unexpectedly succeeded; implicit-TLS branch not taken")
	}
	if !strings.Contains(err.Error(), "tls") {
		t.Errorf("error = %q, want a TLS handshake error", err)
	}
}

// ── Boot-time sender-identity gate (verifySMTPConfig) ────────────────

func TestVerifySMTPConfig_UnsetHostSkipped(t *testing.T) {
	t.Setenv("OZ_SMTP_HOST", "")
	t.Setenv("OZ_SMTP_FROM", "")
	if err := verifySMTPConfig(); err != nil {
		t.Fatalf("unset OZ_SMTP_HOST should skip the check, got error: %v", err)
	}
}

func TestVerifySMTPConfig_UnsetFromFallsBackToDefault(t *testing.T) {
	// When OZ_SMTP_FROM is empty, the code falls back to smtpDefaultFrom
	// and the SMTP probe decides whether the relay accepts it.
	addr, _ := runSMTPServer(t, nil, false) // accepts all senders
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}
	t.Setenv("OZ_SMTP_HOST", host)
	t.Setenv("OZ_SMTP_PORT", port)
	t.Setenv("OZ_SMTP_USER", "")
	t.Setenv("OZ_SMTP_PASSWORD", "")
	t.Setenv("OZ_SMTP_FROM", "")
	if err := verifySMTPConfig(); err != nil {
		t.Fatalf("unset OZ_SMTP_FROM should fall back to default and pass, got: %v", err)
	}
}

func TestVerifySMTPConfig_DefaultFromAccepted(t *testing.T) {
	// The default sender is now a real verified sender on Brevo — the
	// SMTP probe decides whether it works, not a hardcoded rejection.
	// With an unreachable relay, the probe is transient → warn-only.
	addr, _ := runSMTPServer(t, nil, false) // accepts all senders
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}
	t.Setenv("OZ_SMTP_HOST", host)
	t.Setenv("OZ_SMTP_PORT", port)
	t.Setenv("OZ_SMTP_USER", "")
	t.Setenv("OZ_SMTP_PASSWORD", "")
	t.Setenv("OZ_SMTP_FROM", smtpDefaultFrom)
	if err := verifySMTPConfig(); err != nil {
		t.Fatalf("default sender should pass when relay accepts it, got: %v", err)
	}
}

func TestVerifySMTPConfig_AcceptedSenderPasses(t *testing.T) {
	addr, _ := runSMTPServer(t, nil, false)
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}
	t.Setenv("OZ_SMTP_HOST", host)
	t.Setenv("OZ_SMTP_PORT", port)
	t.Setenv("OZ_SMTP_USER", "otp-user")
	t.Setenv("OZ_SMTP_PASSWORD", "otp-pass")
	t.Setenv("OZ_SMTP_FROM", "verified@example.com")
	if err := verifySMTPConfig(); err != nil {
		t.Fatalf("accepted sender should pass the gate, got error: %v", err)
	}
}

func TestVerifySMTPConfig_RejectedSenderFails(t *testing.T) {
	addr, _ := runSMTPServer(t, nil, true) // 550 on MAIL FROM
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}
	t.Setenv("OZ_SMTP_HOST", host)
	t.Setenv("OZ_SMTP_PORT", port)
	t.Setenv("OZ_SMTP_USER", "")
	t.Setenv("OZ_SMTP_PASSWORD", "")
	t.Setenv("OZ_SMTP_FROM", "unverified@example.com")
	err = verifySMTPConfig()
	if err == nil {
		t.Fatal("a permanent relay rejection of the sender must fail fast")
	}
	if !strings.Contains(err.Error(), "permanent rejection") {
		t.Errorf("error = %q, want the permanent-rejection framing", err)
	}
}

func TestVerifySMTPConfig_UnreachableRelayWarnsOnly(t *testing.T) {
	// Grab a port that is guaranteed closed, then release it: dialing it
	// fails fast with connection refused instead of hanging.
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	addr := ln.Addr().String()
	ln.Close()
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}
	t.Setenv("OZ_SMTP_HOST", host)
	t.Setenv("OZ_SMTP_PORT", port)
	t.Setenv("OZ_SMTP_USER", "")
	t.Setenv("OZ_SMTP_PASSWORD", "")
	t.Setenv("OZ_SMTP_FROM", "verified@example.com")
	if err := verifySMTPConfig(); err != nil {
		t.Fatalf("a transient relay outage must warn, not fail boot: %v", err)
	}
}
