use axum::extract::{Path, State};

use crate::Database;
use crate::errors::{Error, JsonErrorResponse};
use crate::handlers::extract::Uid;

pub async fn delete(
    State(db): State<Database>,
    uri: axum::http::Uri,
    Path(id): Path<String>,
    Uid(uid): Uid,
) -> Result<(), JsonErrorResponse> {
    let id_parsed = id.parse().map_err(Error::Id)?;
    let metadata = db.get_metadata(id_parsed).await.map_err(Error::Database)?;

    if let Some(_) = crate::handlers::check_visibility(metadata.is_private, &uri, &id).map_err(|_| Error::Database(wastebin_core::db::Error::NotFound))? {
        return Err(Error::Database(wastebin_core::db::Error::NotFound).into());
    }

    if !metadata.is_private {
        return Err(Error::Database(wastebin_core::db::Error::Delete).into());
    }

    db.delete_for(id_parsed, uid).await.map_err(Error::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::handlers::insert::form::Entry;
    use crate::test_helpers::{Client, StoreCookies};
    use reqwest::StatusCode;

    #[tokio::test]
    async fn delete() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(true)).await;

        let res = client.post_form().form(&Entry {
            visibility: Some("private".to_string()),
            ..Default::default()
        }).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let location = res.headers().get("location").unwrap().to_str()?;
        let id = location.split('/').last().unwrap();

        let res = client.delete(&format!("/s/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::OK);

        let res = client.get(&format!("/s/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
