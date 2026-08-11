//! External-browser commands (ADR #38).
//!
//! `open_product_images_scoped` opens the OS default browser in a new
//! tab at a Google Images search for a product's name (+ brand when
//! set). This is the app's first browser-opening mechanism, exposed
//! through `tauri-plugin-opener` with an https-only, percent-encoded
//! URL built server-side.

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

/// Open a Google Images search for a product in the default browser.
///
/// ADR #38 D2/D3: the query is `name` plus `brand` (when the product
/// has one), percent-encoded server-side. The URL is https-only and
/// never reflects user-controlled input outside the query string.
///
/// Returns `Ok(())` when the opener accepted the request.
#[tauri::command]
pub async fn open_product_images_scoped(
    session_token: String,
    sku: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Resolve the session so only signed-in store sessions can trigger
    // browser opening, and read the product's name/brand from the DB
    // (never trust the frontend with the query text).
    let conn = state.resolve_store(&session_token)?;
    // Scope the DB borrow so `Store` (!Send) is dropped before the await
    // below; only the owned query string crosses the await point.
    let query = {
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = oz_core::db::Store::new(&db);
        let product = store.get_product(&sku)?.ok_or_else(|| AppError::Core {
            sub_kind: oz_core::CoreErrorKind::NotFound,
            message: format!("product {sku} not found"),
        })?;
        build_image_query(&product.product)
    };

    let url = format!(
        "https://www.google.com/search?tbm=isch&q={}",
        urlencoding(&query)
    );

    open_in_browser(&url).await
}

/// Build the Google Images search query: product name plus brand (when set).
fn build_image_query(product: &oz_core::Product) -> String {
    let mut query = product.name.trim().to_owned();
    if let Some(brand) = product
        .brand
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty())
    {
        query.push(' ');
        query.push_str(brand);
    }
    query
}

/// Percent-encode a UTF-8 query for use in a URL query component.
fn urlencoding(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Open a URL in the OS default browser via `tauri-plugin-opener`.
async fn open_in_browser(url: &str) -> Result<(), AppError> {
    tauri_plugin_opener::open_url(url, None::<&str>)
        .map_err(|e| AppError::Internal(format!("opening browser: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoding_encodes_query() {
        assert_eq!(urlencoding("Coca Cola"), "Coca+Cola");
        assert_eq!(urlencoding("Indomie&Co"), "Indomie%26Co");
        assert_eq!(urlencoding("Bakso 100%"), "Bakso+100%25");
        assert_eq!(urlencoding("日本語"), "%E6%97%A5%E6%9C%AC%E8%AA%9E");
    }

    #[test]
    fn urlencoding_keeps_unreserved_chars() {
        assert_eq!(urlencoding("a-z_A.Z~0"), "a-z_A.Z~0");
    }
}
