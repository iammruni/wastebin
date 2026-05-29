use axum::extract::{Path, State};
use axum::response::{IntoResponse, Redirect, Response};

use crate::handlers::extract::{Theme, Uid};
use crate::handlers::html::{ErrorResponse, make_error};
use crate::i18n::Lang;
use crate::{Database, Page};

pub async fn delete(
    State(db): State<Database>,
    State(page): State<Page>,
    uri: axum::http::Uri,
    Path(id): Path<String>,
    Uid(uid): Uid,
    theme: Option<Theme>,
    lang: Lang,
) -> Result<Response, ErrorResponse> {
    async {
        let id_parsed = id.parse()?;
        let metadata = db.get_metadata(id_parsed).await?;

        if let Some(redirect) = crate::handlers::check_visibility(metadata.is_private, &uri, &id)? {
            return Ok(redirect.into_response());
        }

        if !metadata.is_private {
            return Err(crate::Error::Database(wastebin_core::db::Error::Delete));
        }

        db.delete_for(id_parsed, uid).await?;
        Ok(Redirect::to("/").into_response())
    }
    .await
    .map_err(|err| make_error(err, page.clone(), theme, lang))
}

#[cfg(test)]
mod tests {
    use crate::handlers::insert::form::Entry;
    use crate::test_helpers::{Client, StoreCookies};
    use reqwest::StatusCode;

    #[tokio::test]
    async fn delete_via_link() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(true)).await;

        let res = client.post_form().form(&Entry {
            visibility: Some("private".to_string()),
            ..Default::default()
        }).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let location = res.headers().get("location").unwrap().to_str()?;
        let id = location.split('/').last().unwrap();

        let res = client.post(&format!("/s/delete/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let res = client.get(&format!("/s/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
