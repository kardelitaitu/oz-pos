//! User profile data contract (ADR #35 D6 / spec 0049).
//!
//! The `users` table gains the profile columns in migration 130. This module
//! carries the contract: what is mandatory-at-creation, how each field is
//! validated, and the incomplete-profile derivation. Columns are nullable in
//! SQL — "mandatory" is enforced at creation with field-specific errors, and
//! legacy rows (or direct-SQL inserts) enter the incomplete-profile state
//! instead of being rejected.
//!
//! The 9 mandatory-at-creation items: username + full name (both already on
//! `users`, enforced by `create_user`) plus the 8 profile fields below. The
//! D6 not-collected fields (gender, religion, marital status, ethnicity,
//! blood type, bank account, shift/availability) never appear here.

use rusqlite::{OptionalExtension, params};

use crate::error::CoreError;

use super::Store;

/// The user profile fields added by migration 130.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserProfile {
    // ── Required at creation (nullable in SQL) ──────────────────────
    /// ISO date (`YYYY-MM-DD`), never in the future.
    pub date_of_birth: Option<String>,
    /// Phone in E.164 form (`+<country><number>`).
    pub phone: Option<String>,
    /// `"ssn"` (US) or `"nik"` (Indonesian KTP).
    pub national_id_type: Option<String>,
    /// The national id — 9 digits for `ssn`, 16 for `nik`.
    pub national_id: Option<String>,
    /// Lowercase email, unique when present.
    pub email: Option<String>,
    /// Monthly take-home pay in i64 minor units, strictly positive.
    pub monthly_take_home_minor: Option<i64>,
    /// Emergency contact's name (required at creation).
    pub emergency_contact_name: Option<String>,
    /// Emergency contact's phone (required at creation).
    pub emergency_contact_phone: Option<String>,
    // ── Optional ─────────────────────────────────────────────────────
    /// Job title (free text), stable string slot — never affects completeness.
    pub job_title: String,
    /// Free-text notes, stable string slot — never affects completeness.
    pub notes: String,
    /// Street address, optional.
    pub address: Option<String>,
    /// UI language preference, optional.
    pub language: Option<String>,
    /// Avatar reference (path/URL), optional.
    pub avatar: Option<String>,
    /// Tax identification number, optional (distinct from the national id).
    pub tax_id: Option<String>,
    /// Expiry of the national id document (`YYYY-MM-DD`), optional.
    pub national_id_expires_at: Option<String>,
    /// Relationship of the emergency contact (e.g. "spouse"), optional.
    pub emergency_contact_relationship: Option<String>,
    /// Hire date (`YYYY-MM-DD`), optional.
    pub hire_date: Option<String>,
}

impl UserProfile {
    /// Whether all 8 profile-side required fields are present (username +
    /// full name are enforced by `create_user`). A missing required field
    /// means the user is in the incomplete-profile state.
    pub fn is_complete(&self) -> bool {
        self.date_of_birth.is_some()
            && self.phone.is_some()
            && self.national_id_type.is_some()
            && self.national_id.is_some()
            && self.email.is_some()
            && self.monthly_take_home_minor.is_some()
            && self.emergency_contact_name.is_some()
            && self.emergency_contact_phone.is_some()
    }

    /// Field-specific validation for creation and profile updates: every
    /// required field present, `national_id` shaped per its type (ssn 9 /
    /// nik 16), email well-formed, phone E.164, DOB not in the future,
    /// monthly pay strictly positive.
    pub fn validate(&self) -> Result<(), CoreError> {
        // 1. All 8 required fields present, reported in a fixed order.
        if self.date_of_birth.is_none() {
            return Err(validation("date_of_birth", "date of birth is required"));
        }
        if self.phone.is_none() {
            return Err(validation("phone", "phone number is required"));
        }
        if self.national_id_type.is_none() {
            return Err(validation(
                "national_id_type",
                "national id type is required",
            ));
        }
        if self.national_id.is_none() {
            return Err(validation("national_id", "national id is required"));
        }
        if self.email.is_none() {
            return Err(validation("email", "email address is required"));
        }
        if self.monthly_take_home_minor.is_none() {
            return Err(validation(
                "monthly_take_home_minor",
                "monthly take-home pay is required",
            ));
        }
        if self.emergency_contact_name.is_none() {
            return Err(validation(
                "emergency_contact_name",
                "emergency contact name is required",
            ));
        }
        if self.emergency_contact_phone.is_none() {
            return Err(validation(
                "emergency_contact_phone",
                "emergency contact phone is required",
            ));
        }

        // 2. National id type: ssn (US) or nik (Indonesian KTP) only.
        let id_type = self.national_id_type.as_deref().unwrap();
        if id_type != "ssn" && id_type != "nik" {
            return Err(validation(
                "national_id_type",
                "national id type must be 'ssn' or 'nik'",
            ));
        }

        // 3. National id shape: exactly 9 digits (ssn) or 16 (nik).
        let id = self.national_id.as_deref().unwrap();
        let expected = if id_type == "ssn" { 9 } else { 16 };
        if id.len() != expected || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Err(validation(
                "national_id",
                format!("national id must be {expected} digits for {id_type}"),
            ));
        }

        // 4. Email: local@domain.tld, no whitespace.
        if let Some(email) = self.email.as_deref() {
            let valid = match email.split_once('@') {
                Some((local, domain)) => {
                    !local.is_empty()
                        && domain.contains('.')
                        && !email.chars().any(char::is_whitespace)
                }
                None => false,
            };
            if !valid {
                return Err(validation("email", "email address is not well-formed"));
            }
        }

        // 5. Phone: E.164 — `+` then 7..=15 digits.
        if let Some(phone) = self.phone.as_deref() {
            let digits = phone.strip_prefix('+');
            let valid = matches!(digits, Some(d) if (7..=14).contains(&d.len()) && d.bytes().all(|b| b.is_ascii_digit()));
            if !valid {
                return Err(validation(
                    "phone",
                    "phone must be E.164 (+country number, max 15 digits)",
                ));
            }
        }

        // 6. Date of birth: ISO date, never in the future.
        if let Some(dob) = self.date_of_birth.as_deref() {
            let today = chrono::Utc::now().date_naive();
            match chrono::NaiveDate::parse_from_str(dob, "%Y-%m-%d") {
                Ok(parsed) if parsed <= today => {}
                _ => {
                    return Err(validation(
                        "date_of_birth",
                        "date of birth must be a valid past date (YYYY-MM-DD)",
                    ));
                }
            }
        }

        // 7. Monthly take-home pay: strictly positive minor units.
        if let Some(pay) = self.monthly_take_home_minor
            && pay <= 0
        {
            return Err(validation(
                "monthly_take_home_minor",
                "monthly take-home pay must be positive",
            ));
        }

        Ok(())
    }
}

fn validation(field: &'static str, message: impl Into<String>) -> CoreError {
    CoreError::Validation {
        field,
        message: message.into(),
    }
}

/// The profile columns of `users`, in parameter order (SELECT list).
const PROFILE_COLUMNS: &str = "date_of_birth, phone, national_id_type, national_id, email, \
     monthly_take_home_minor, emergency_contact_name, emergency_contact_phone, job_title, notes, \
     address, language, avatar, tax_id, national_id_expires_at, emergency_contact_relationship, \
     hire_date";

/// The same columns as `col = ?N` assignments, in parameter order (UPDATE).
const PROFILE_ASSIGNMENTS: &str = "date_of_birth=?1, phone=?2, national_id_type=?3, \
     national_id=?4, email=?5, monthly_take_home_minor=?6, emergency_contact_name=?7, \
     emergency_contact_phone=?8, job_title=?9, notes=?10, address=?11, language=?12, avatar=?13, \
     tax_id=?14, national_id_expires_at=?15, emergency_contact_relationship=?16, hire_date=?17";

impl Store<'_> {
    /// Load a user's profile, or `None` when the user does not exist.
    pub fn get_user_profile(&self, user_id: &str) -> Result<Option<UserProfile>, CoreError> {
        let sql = format!("SELECT {PROFILE_COLUMNS} FROM users WHERE id = ?1");
        let profile = self
            .conn
            .query_row(&sql, params![user_id], |row| {
                Ok(UserProfile {
                    date_of_birth: row.get(0)?,
                    phone: row.get(1)?,
                    national_id_type: row.get(2)?,
                    national_id: row.get(3)?,
                    email: row.get(4)?,
                    monthly_take_home_minor: row.get(5)?,
                    emergency_contact_name: row.get(6)?,
                    emergency_contact_phone: row.get(7)?,
                    job_title: row.get(8)?,
                    notes: row.get(9)?,
                    address: row.get(10)?,
                    language: row.get(11)?,
                    avatar: row.get(12)?,
                    tax_id: row.get(13)?,
                    national_id_expires_at: row.get(14)?,
                    emergency_contact_relationship: row.get(15)?,
                    hire_date: row.get(16)?,
                })
            })
            .optional()?;
        Ok(profile)
    }

    /// Create a user with the full profile contract: validates the 9
    /// mandatory fields, inserts the user + default global assignment, then
    /// writes the profile columns — all in one transaction so a profile
    /// conflict (duplicate email / national id) rolls the user back instead
    /// of leaving a partial row.
    pub fn create_user_with_profile(
        &self,
        username: &str,
        pin_hash: &str,
        display_name: &str,
        role_id: &str,
        profile: &UserProfile,
    ) -> Result<crate::User, CoreError> {
        profile.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        let store = Store::new(&tx);
        let user = store.create_user(username, pin_hash, display_name, role_id)?;
        store.update_user_profile(&user.id, profile)?;
        tx.commit()?;
        Ok(user)
    }

    /// Update a user's profile columns (validated). Duplicate email /
    /// national_id surface as field-level conflicts via the unique indexes.
    pub fn update_user_profile(
        &self,
        user_id: &str,
        profile: &UserProfile,
    ) -> Result<(), CoreError> {
        profile.validate()?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let sql =
            format!("UPDATE users SET {PROFILE_ASSIGNMENTS}, updated_at = ?18 WHERE id = ?19");
        let result = self.conn.execute(
            &sql,
            params![
                &profile.date_of_birth,
                &profile.phone,
                &profile.national_id_type,
                &profile.national_id,
                &profile.email,
                &profile.monthly_take_home_minor,
                &profile.emergency_contact_name,
                &profile.emergency_contact_phone,
                &profile.job_title,
                &profile.notes,
                &profile.address,
                &profile.language,
                &profile.avatar,
                &profile.tax_id,
                &profile.national_id_expires_at,
                &profile.emergency_contact_relationship,
                &profile.hire_date,
                now,
                user_id
            ],
        );
        match result {
            Err(rusqlite::Error::SqliteFailure(f, msg))
                if f.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                let msg = msg.unwrap_or_default().to_lowercase();
                if msg.contains("users.email") {
                    return Err(CoreError::Conflict {
                        entity: "user",
                        field: "email",
                    });
                }
                if msg.contains("users.national_id") {
                    return Err(CoreError::Conflict {
                        entity: "user",
                        field: "national_id",
                    });
                }
                Err(rusqlite::Error::SqliteFailure(f, None).into())
            }
            Err(e) => Err(e.into()),
            Ok(0) => Err(CoreError::NotFound {
                entity: "user",
                id: user_id.to_owned(),
            }),
            Ok(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn complete_profile() -> UserProfile {
        UserProfile {
            date_of_birth: Some("1990-05-14".into()),
            phone: Some("+14155550123".into()),
            national_id_type: Some("ssn".into()),
            national_id: Some("123456789".into()),
            email: Some("alice@example.com".into()),
            monthly_take_home_minor: Some(5_000_000),
            emergency_contact_name: Some("Bob".into()),
            emergency_contact_phone: Some("+14155550987".into()),
            ..Default::default()
        }
    }

    #[test]
    fn is_complete_true_when_all_required_present() {
        assert!(complete_profile().is_complete());
    }

    #[test]
    fn is_complete_false_when_any_required_missing() {
        // Every one of the 8 profile-side required fields, missing one at a
        // time, must yield an incomplete profile (ADR #35 D6 semantics:
        // legacy rows are flagged, never rejected).
        let mut profile = complete_profile();
        let fields = [
            "date_of_birth",
            "phone",
            "national_id_type",
            "national_id",
            "email",
            "monthly_take_home_minor",
            "emergency_contact_name",
            "emergency_contact_phone",
        ];
        for field in fields {
            let mut p = profile.clone();
            match field {
                "date_of_birth" => p.date_of_birth = None,
                "phone" => p.phone = None,
                "national_id_type" => p.national_id_type = None,
                "national_id" => p.national_id = None,
                "email" => p.email = None,
                "monthly_take_home_minor" => p.monthly_take_home_minor = None,
                "emergency_contact_name" => p.emergency_contact_name = None,
                "emergency_contact_phone" => p.emergency_contact_phone = None,
                _ => unreachable!(),
            }
            assert!(
                !p.is_complete(),
                "{field} missing must make the profile incomplete"
            );
        }
        // Optionals never affect completeness.
        profile.job_title = "Cashier".into();
        profile.notes = "notes".into();
        assert!(profile.is_complete());
    }

    #[test]
    fn validate_rejects_each_missing_required_field() {
        let profile = complete_profile();
        let missing = [
            "date_of_birth",
            "phone",
            "national_id_type",
            "national_id",
            "email",
            "monthly_take_home_minor",
            "emergency_contact_name",
            "emergency_contact_phone",
        ];
        for field in missing {
            let mut p = profile.clone();
            match field {
                "date_of_birth" => p.date_of_birth = None,
                "phone" => p.phone = None,
                "national_id_type" => p.national_id_type = None,
                "national_id" => p.national_id = None,
                "email" => p.email = None,
                "monthly_take_home_minor" => p.monthly_take_home_minor = None,
                "emergency_contact_name" => p.emergency_contact_name = None,
                "emergency_contact_phone" => p.emergency_contact_phone = None,
                _ => unreachable!(),
            }
            let err = p.validate().unwrap_err();
            match err {
                CoreError::Validation { field: f, .. } => {
                    assert_eq!(f, field, "missing {field} must report that field")
                }
                other => panic!("expected Validation for {field}, got {other:?}"),
            }
        }
    }

    #[test]
    fn validate_accepts_complete_profile() {
        assert!(complete_profile().validate().is_ok());
    }

    #[test]
    fn validate_national_id_shape_per_type() {
        // ssn: exactly 9 digits.
        for bad in ["12345678", "1234567890", "12345678a", "abcdefghi"] {
            let mut p = complete_profile();
            p.national_id = Some(bad.into());
            let err = p.validate().unwrap_err();
            assert!(
                matches!(
                    err,
                    CoreError::Validation {
                        field: "national_id",
                        ..
                    }
                ),
                "ssn {bad:?} must be rejected with a national_id field error"
            );
        }
        // nik: exactly 16 digits.
        let mut p = complete_profile();
        p.national_id_type = Some("nik".into());
        p.national_id = Some("3201010101010001".into());
        assert!(p.validate().is_ok(), "16-digit nik must pass");
        let mut p = complete_profile();
        p.national_id_type = Some("nik".into());
        p.national_id = Some("3201010101".into());
        let err = p.validate().unwrap_err();
        assert!(
            matches!(
                err,
                CoreError::Validation {
                    field: "national_id",
                    ..
                }
            ),
            "10-digit nik must be rejected"
        );
        // Unknown type.
        let mut p = complete_profile();
        p.national_id_type = Some("passport".into());
        let err = p.validate().unwrap_err();
        assert!(
            matches!(
                err,
                CoreError::Validation {
                    field: "national_id_type",
                    ..
                }
            ),
            "unknown national_id_type must be rejected"
        );
    }

    #[test]
    fn validate_email_and_phone_and_dob_and_pay() {
        for bad_email in ["not-an-email", "@nope", "a@b", "a b@c.com"] {
            let mut p = complete_profile();
            p.email = Some(bad_email.into());
            assert!(
                matches!(
                    p.validate(),
                    Err(CoreError::Validation { field: "email", .. })
                ),
                "email {bad_email:?} must be rejected"
            );
        }
        for bad_phone in ["4155550123", "+1", "abc", "+141555501234567"] {
            let mut p = complete_profile();
            p.phone = Some(bad_phone.into());
            assert!(
                matches!(
                    p.validate(),
                    Err(CoreError::Validation { field: "phone", .. })
                ),
                "phone {bad_phone:?} must be rejected"
            );
        }
        // DOB in the future.
        let mut p = complete_profile();
        p.date_of_birth = Some("2999-01-01".into());
        assert!(matches!(
            p.validate(),
            Err(CoreError::Validation {
                field: "date_of_birth",
                ..
            })
        ));
        // Monthly pay must be strictly positive.
        for bad_pay in [0, -1, -5_000_000] {
            let mut p = complete_profile();
            p.monthly_take_home_minor = Some(bad_pay);
            assert!(
                matches!(
                    p.validate(),
                    Err(CoreError::Validation {
                        field: "monthly_take_home_minor",
                        ..
                    })
                ),
                "pay {bad_pay} must be rejected"
            );
        }
    }

    #[test]
    fn create_with_profile_writes_columns_and_roundtrips() {
        let conn = migrations::fresh_db();
        conn.execute_batch(
            "INSERT INTO roles (id, name, permissions) VALUES
                 ('role-staff', 'staff', '[\"sales:view\"]');",
        )
        .unwrap();
        let store = Store::new(&conn);
        let profile = complete_profile();
        let user = store
            .create_user_with_profile("alice", "hash", "Alice", "role-staff", &profile)
            .unwrap();
        let loaded = store
            .get_user_profile(&user.id)
            .unwrap()
            .expect("profile must round-trip");
        assert_eq!(loaded, profile);
        assert!(loaded.is_complete());
    }

    #[test]
    fn create_with_profile_rejects_duplicate_email_and_national_id() {
        let conn = migrations::fresh_db();
        conn.execute_batch(
            "INSERT INTO roles (id, name, permissions) VALUES
                 ('role-staff', 'staff', '[\"sales:view\"]');",
        )
        .unwrap();
        let store = Store::new(&conn);
        store
            .create_user_with_profile("alice", "hash", "Alice", "role-staff", &complete_profile())
            .unwrap();

        // Duplicate email.
        let mut dup_email = complete_profile();
        dup_email.national_id = Some("987654321".into()); // different id, same email
        let err = store
            .create_user_with_profile("bob", "hash", "Bob", "role-staff", &dup_email)
            .unwrap_err();
        assert!(
            matches!(err, CoreError::Conflict { field: "email", .. }),
            "duplicate email must be a field-level conflict, got {err:?}"
        );

        // Duplicate national_id.
        let mut dup_id = complete_profile();
        dup_id.email = Some("bob@example.com".into()); // different email, same id
        let err = store
            .create_user_with_profile("bob", "hash", "Bob", "role-staff", &dup_id)
            .unwrap_err();
        assert!(
            matches!(
                err,
                CoreError::Conflict {
                    field: "national_id",
                    ..
                }
            ),
            "duplicate national_id must be a field-level conflict, got {err:?}"
        );
    }

    #[test]
    fn legacy_create_user_leaves_incomplete_profile() {
        let conn = migrations::fresh_db();
        conn.execute_batch(
            "INSERT INTO roles (id, name, permissions) VALUES
                 ('role-staff', 'staff', '[\"sales:view\"]');",
        )
        .unwrap();
        let store = Store::new(&conn);
        let user = store
            .create_user("legacy", "hash", "Legacy", "role-staff")
            .unwrap();
        let profile = store
            .get_user_profile(&user.id)
            .unwrap()
            .expect("every user has a profile row (all-NULL for legacy)");
        assert!(
            !profile.is_complete(),
            "legacy users are incomplete, never rejected"
        );
    }

    #[test]
    fn update_user_profile_roundtrips() {
        let conn = migrations::fresh_db();
        conn.execute_batch(
            "INSERT INTO roles (id, name, permissions) VALUES
                 ('role-staff', 'staff', '[\"sales:view\"]');",
        )
        .unwrap();
        let store = Store::new(&conn);
        let user = store
            .create_user("alice", "hash", "Alice", "role-staff")
            .unwrap();
        let mut profile = complete_profile();
        profile.email = Some("alice.new@example.com".into());
        store.update_user_profile(&user.id, &profile).unwrap();
        let loaded = store.get_user_profile(&user.id).unwrap().unwrap();
        assert_eq!(loaded, profile);
        assert!(loaded.is_complete());
    }

    #[test]
    fn get_user_profile_returns_none_for_unknown_user() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        assert!(store.get_user_profile("no-such-user").unwrap().is_none());
    }
}
