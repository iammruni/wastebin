pub mod delete;
pub mod download;
pub mod extract;
pub mod html;
pub mod insert;
pub mod raw;
pub mod robots;
pub mod theme;

use axum_extra::extract::cookie::{Cookie, SameSite};

/// Build a cookie with secure defaults: `HttpOnly`, `SameSite=Strict`, `Path=/`.
pub(crate) fn cookie(name: &str, value: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(name.to_owned(), value);
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Strict);
    cookie.set_path("/");
    cookie
}
pub(crate) fn check_visibility(
    is_private: bool,
    uri: &axum::http::Uri,
    id: &str,
) -> Result<Option<axum::response::Redirect>, crate::errors::Error> {
    let path = uri.path();
    if path.starts_with("/s/") {
        if !is_private {
            return Err(crate::errors::Error::Database(wastebin_core::db::Error::NotFound));
        }
    } else if path.starts_with("/p/") {
        if is_private {
            return Err(crate::errors::Error::Database(wastebin_core::db::Error::NotFound));
        }
    } else {
        let prefix = if is_private { "/s" } else { "/p" };
        let redirect_path = if path.starts_with("/burn/") {
            format!("{prefix}/burn/{id}")
        } else if path.starts_with("/md/") {
            format!("{prefix}/md/{id}")
        } else if path.starts_with("/qr/") {
            format!("{prefix}/qr/{id}")
        } else if path.starts_with("/dl/") {
            format!("{prefix}/dl/{id}")
        } else if path.starts_with("/raw/") {
            format!("{prefix}/raw/{id}")
        } else if path.starts_with("/delete/") {
            format!("{prefix}/delete/{id}")
        } else {
            format!("{prefix}/{id}")
        };
        return Ok(Some(axum::response::Redirect::temporary(&redirect_path)));
    }
    Ok(None)
}
