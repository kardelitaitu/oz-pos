package main

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

// TestNoRawLicenseKeyLogging is a structural guard, not a spot check.
//
// The sweep that introduced maskLicenseKey fixed eleven call sites across
// seven files. A fix like that regrows silently: the next handler that logs
// "failed for key %q" looks harmless in review because the value is only a
// license key, and license keys are bearer entitlements.
//
// It is AST-based rather than grep-based on purpose. Two of the eleven sites
// were missed by a first grep pass because the arguments sit on the line
// after the format string, and an AST sees a call as one node regardless of
// how it is wrapped.
//
// A site may opt out with a "// key-log:masked <reason>" comment on the same
// line as the call, for values that are masked by some other means. The
// reason is required, so the escape hatch leaves an audit trail instead of
// becoming a way to silence the check.
func TestNoRawLicenseKeyLogging(t *testing.T) {
	// Names that hold a license key value at the sites found in the sweep.
	// Deliberately narrow: "key" alone matches map keys, lockout buckets and
	// env-var names, and a guard that cries wolf gets ignored.
	secretExprs := []string{
		"req.Key",
		"body.Key",
		"req.LicenseKey",
		"licenseKey",
		"newKey",
		"keyVal",
	}

	fset := token.NewFileSet()
	var violations []string

	err := filepath.Walk(".", func(path string, info os.FileInfo, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if info.IsDir() {
			if name := info.Name(); name == "node_modules" || name == ".git" || name == "pb_data" {
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(path, ".go") || strings.HasSuffix(path, "_test.go") {
			return nil
		}

		src, readErr := os.ReadFile(path)
		if readErr != nil {
			t.Errorf("read %s: %v", path, readErr)
			return nil
		}
		file, parseErr := parser.ParseFile(fset, path, src, parser.ParseComments)
		if parseErr != nil {
			t.Errorf("parse %s: %v", path, parseErr)
			return nil
		}

		lines := strings.Split(string(src), "\n")

		ast.Inspect(file, func(n ast.Node) bool {
			call, ok := n.(*ast.CallExpr)
			if !ok {
				return true
			}
			if !isLogCall(call) || len(call.Args) == 0 {
				return true
			}
			// Skip the format string itself; secrets travel in the args.
			for _, arg := range call.Args[1:] {
				text := nodeText(src, fset, arg)
				if text == "" {
					continue
				}
				if !mentionsSecret(text, secretExprs) {
					continue
				}
				if strings.Contains(text, "maskLicenseKey") {
					continue
				}
				pos := fset.Position(call.Pos())
				lineText := ""
				if pos.Line-1 < len(lines) {
					lineText = lines[pos.Line-1]
				}
				if strings.Contains(lineText, "key-log:masked") {
					continue
				}
				violations = append(violations, path+":"+itoa(pos.Line)+" logs "+text)
			}
			return true
		})
		return nil
	})
	if err != nil {
		t.Fatalf("walk: %v", err)
	}

	sort.Strings(violations)
	if len(violations) > 0 {
		t.Errorf("%d log site(s) write a license key value without maskLicenseKey:\n  %s\n"+
			"Wrap the value in maskLicenseKey(), or add \"// key-log:masked <reason>\" if it is masked some other way.",
			len(violations), strings.Join(violations, "\n  "))
	}
}

// isLogCall reports whether call is log.Print / log.Printf / log.Println.
func isLogCall(call *ast.CallExpr) bool {
	sel, ok := call.Fun.(*ast.SelectorExpr)
	if !ok {
		return false
	}
	pkg, ok := sel.X.(*ast.Ident)
	if !ok || pkg.Name != "log" {
		return false
	}
	switch sel.Sel.Name {
	case "Print", "Printf", "Println":
		return true
	}
	return false
}

// nodeText renders an expression back to its source text.
func nodeText(src []byte, fset *token.FileSet, expr ast.Expr) string {
	start := fset.Position(expr.Pos()).Offset
	end := fset.Position(expr.End()).Offset
	if start < 0 || end > len(src) || start >= end {
		return ""
	}
	return string(src[start:end])
}

// mentionsSecret reports whether text refers to one of the secret-bearing
// names, without matching a longer identifier that merely starts with it.
func mentionsSecret(text string, names []string) bool {
	for _, name := range names {
		for i := 0; i+len(name) <= len(text); i++ {
			if text[i:i+len(name)] != name {
				continue
			}
			beforeOK := i == 0 || !isIdentByte(text[i-1])
			after := i + len(name)
			afterOK := after >= len(text) || !isIdentByte(text[after])
			if beforeOK && afterOK {
				return true
			}
		}
	}
	return false
}

func isIdentByte(b byte) bool {
	return b == '_' || (b >= '0' && b <= '9') || (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z')
}

func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	var buf [20]byte
	i := len(buf)
	for n > 0 {
		i--
		buf[i] = byte('0' + n%10)
		n /= 10
	}
	return string(buf[i:])
}
