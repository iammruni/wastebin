use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use qrcodegen::QrCode;
use url::Url;

use crate::cache::Key;
use crate::handlers::extract::{Theme, Uid};
use crate::handlers::html::paste::is_markdown_ext;
use crate::handlers::html::{ErrorResponse, make_error};
use crate::i18n::Lang;
use crate::{Error, Page};
use wastebin_core::db::Database;
use wastebin_core::expiration::Expiration;

/// GET handler for a QR page.
pub async fn get(
    State(page): State<Page>,
    State(db): State<Database>,
    uri: axum::http::Uri,
    Path(id): Path<String>,
    uid: Option<Uid>,
    theme: Option<Theme>,
    lang: Lang,
) -> Result<Response, ErrorResponse> {
    async {
        let key: Key = id.parse()?;

        let metadata = db.get_metadata(key.id).await?;

        if let Some(redirect) = super::super::check_visibility(metadata.is_private, &uri, &id)? {
            return Ok(redirect.into_response());
        }

        let code = {
            let page = page.clone();

            tokio::task::spawn_blocking(move || code_from(&page.base_url, &id))
                .await
                .map_err(Error::from)??
        };

        let owner_uid = metadata.uid;
        let title = metadata.title.clone();
        let expiration = metadata.expiration;
        let is_private = metadata.is_private;

        let can_delete = uid
            .zip(owner_uid)
            .is_some_and(|(Uid(user_uid), owner_uid)| user_uid == owner_uid);

        let is_markdown = is_markdown_ext(key.ext.as_deref());

        let qr = Qr {
            page: page.clone(),
            theme: theme.clone(),
            lang,
            key,
            can_delete,
            is_available: true,
            code,
            title,
            expiration,
            is_markdown,
            is_private,
        };
        Ok(qr.into_response())
    }
    .await
    .map_err(|err| make_error(err, page, theme, lang))
}

/// Paste view showing the formatted paste as well as a bunch of links.
#[derive(Template, WebTemplate)]
#[template(path = "qr.html", escape = "none")]
pub(crate) struct Qr {
    page: Page,
    theme: Option<Theme>,
    lang: Lang,
    key: Key,
    can_delete: bool,
    is_available: bool,
    is_markdown: bool,
    code: qrcodegen::QrCode,
    title: Option<String>,
    expiration: Option<Expiration>,
    is_private: bool,
}

impl Qr {
    fn dark_modules(&self) -> Vec<(i32, i32)> {
        dark_modules(&self.code)
    }
}

pub fn code_from(url: &Url, id: &str) -> Result<QrCode, Error> {
    Ok(QrCode::encode_text(
        url.join(id)?.as_str(),
        qrcodegen::QrCodeEcc::High,
    )?)
}

/// Return module coordinates that are dark.
pub fn dark_modules(code: &QrCode) -> Vec<(i32, i32)> {
    let size = code.size();
    (0..size)
        .flat_map(|x| (0..size).map(move |y| (x, y)))
        .filter(|(x, y)| code.get_module(*x, *y))
        .collect()
}
