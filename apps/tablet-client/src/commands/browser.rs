//! External-browser commands (ADR #38).
//!
//! `open_product_images` opens the OS default browser in a new tab at a
//! Google Images search for a product's name (+ brand when set), via
//! `tauri-plugin-opener` with an https-only, percent-encoded URL built
//! server-side. Tablet variant of the desktop `open_product_images_scoped`
//! (global-db, non-scoped — matches the tablet's other product commands).

use tauri::{State, command};

use crate::error::AppError;
use crate::state::AppState;

/// Open a Google Images search for a product in the default browser.
///
/// ADR #38 D2/D3: the query is `name` plus `brand` (when the product
/// has one), percent-encoded server-side. The URL is https-only.
#[command]
pub async fn open_product_images(sku: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let query = {
        let db = state.db.lock().await;
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

#[cfg(test)] #[path = "browser_tests.rs"] mod tests;
