"""Search for Square/QRIS sandbox credential references."""
import os

results = {}
targets = [
    "square_access_token", "square_api_key", "midtrans_server_key",
    "midtrans_sandbox", "square_sandbox", "EAAA_", "MIDTRANS_SERVER_KEY",
    "MIDTRANS_CLIENT_KEY", "sk_test", "sandbox_e2e"
]
exts = {".rs", ".toml", ".yml", ".yaml", ".md", ".env", ".env.example"}

for root, dirs, files in os.walk("."):
    parts = root.replace("\\", "/").split("/")
    if any(skip in parts for skip in ("target", ".git", "node_modules")):
        continue
    for f in files:
        if any(f.endswith(e) for e in exts):
            fp = os.path.join(root, f)
            try:
                with open(fp, "r", encoding="utf-8", errors="ignore") as fh:
                    content = fh.read().lower()
                for term in targets:
                    if term.lower() in content:
                        results[fp] = results.get(fp, set()) | {term}
            except:
                pass

if results:
    print("=== FOUND SQUARE/QRIS SANDBOX CREDENTIAL REFERENCES ===")
    for fp, terms in sorted(results.items()):
        print(f'\n{os.path.normpath(fp)}:')
        for t in sorted(terms):
            print(f"  - {t}")
else:
    print("No square/qris sandbox credential references found outside expected files.")
