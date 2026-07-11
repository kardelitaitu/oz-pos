# PCI-DSS Compliance Checklist

> **Status:** Planning / Review
> This checklist documents OZ-POS's alignment with PCI Data Security Standard v4.0 requirements applicable to a point-of-sale application.

## Scope

OZ-POS processes, transmits, and stores cardholder data when processing credit/debit card payments. This checklist covers the application-level requirements.

---

## Build and Maintain a Secure Network

| Requirement | Status | Notes |
|-------------|--------|-------|
| 1.1.1 Firewall between POS terminals and untrusted networks | N/A | Handled by network infrastructure |
| 1.2.2 No direct public access between cardholder data environment and internet | N/A | Handled by infrastructure |
| 1.3.2 DMZ for public-facing services | N/A | OZ-POS API can be deployed behind reverse proxy |

## Protect Cardholder Data

| Requirement | Status | Notes |
|-------------|--------|-------|
| 3.2.1 Do not store full PAN, CVV, or PIN after authorization | ✅ Application | OZ-POS never stores PAN, CVV, or PIN. Payment tokens only. |
| 3.3.0 Mask PAN when displayed (first 6 + last 4) | ✅ Implemented | `mask_pan()` helper in `oz-security` |
| 3.4.0 Render PAN unreadable when stored (encryption, tokenization) | ✅ Implemented | `oz-security` provides encryption helpers |
| 3.5.1 Document key management procedures | 📋 Planned | Key rotation policy in progress |
| 3.6.1 Secure cryptographic key storage | ✅ Implemented | OS keyring via `oz-security::Keyring` |

## Maintain a Vulnerability Management Program

| Requirement | Status | Notes |
|-------------|--------|-------|
| 5.2.1 Deploy anti-malware on POS systems | N/A | OS-level responsibility |
| 6.2.2 Use only secure versions of frameworks and libraries | ✅ CI | `cargo audit` runs weekly via security workflow |
| 6.3.1 Security patches applied within 1 month | 🟡 CI/CD | Automated patch tracking via dependabot + cargo audit |
| 6.4.1 Change control process for all production systems | 📋 Planned | Release process documented in `RELEASE.md` |

## Implement Strong Access Control Measures

| Requirement | Status | Notes |
|-------------|--------|-------|
| 7.1.1 Restrict access to cardholder data by business need-to-know | ✅ Implemented | RBAC via `StaffRoles` feature (owner/manager/cashier) |
| 7.2.1 Role-based access control matrix | ✅ Implemented | See `oz-core::user` for permission model |
| 8.2.1 Unique user IDs for all personnel | ✅ Implemented | Each cashier has unique login |
| 8.3.1 Secure authentication (multi-factor where possible) | 📋 Planned | PIN-based auth → MFA in Phase 3 |
| 8.5.1 Manage user identities and access | ✅ Implemented | User management via Staff Management UI |
| 9.1.1 Physical security of POS terminals | N/A | OS-level responsibility |

## Regularly Monitor and Test Networks

| Requirement | Status | Notes |
|-------------|--------|-------|
| 10.2.1 Audit log captures user ID, event type, date/time, success/failure | ✅ Implemented | `AuditLog` feature with immutable append-only log |
| 10.3.1 Audit logs cannot be modified | ✅ Implemented | Immutable audit log (no UPDATE/DELETE) |
| 10.4.1 Audit log review at least daily | 📋 Planned | Dashboard notification for unreviewed events |
| 10.7.1 Log retention for at least 12 months | 📋 Planned | Log rotation + retention in `oz-logging` |
| 11.3.1 External vulnerability scans quarterly | N/A | Infrastructure-level |
| 11.3.2 Internal vulnerability scans quarterly | N/A | Infrastructure-level |
| 11.5.1 Intrusion detection/deployment | N/A | Infrastructure-level |

## Maintain an Information Security Policy

| Requirement | Status | Notes |
|-------------|--------|-------|
| 12.1.1 Information security policy | ✅ Implemented | See `docs/security/` and `architecture/whitepaper.md` |
| 12.3.1 Usage policies for critical technologies | ✅ Implemented | `AGENTS.md` coding standards |
| 12.5.1 Incident response plan | 📋 Planned | Incident response template in `docs/security/` |
| 12.8.1 Manage service providers with access to CDE | N/A | No third-party service providers |

---

## Quick Reference — OZ-POS PCI-DSS Features

| Feature | Implementation |
|---------|----------------|
| **PAN masking** | `oz_security::mask_pan()` — shows first 6 + last 4 digits |
| **Encrypted storage** | AES-256-GCM via `oz-security` (future: KEK in OS keyring) |
| **Key management** | OS-level keyring (`oz_security::Keyring`) |
| **RBAC** | `StaffRoles` feature with owner/manager/cashier roles |
| **Audit logging** | `AuditLog` feature — immutable, append-only |
| **Dependency scanning** | `cargo audit` weekly via GitHub Actions |
| **Coding standards** | `AGENTS.md` with security rules |

> **Last updated:** 2026-06-28
