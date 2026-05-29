use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response, Redirect};

use crate::auth::AdminAuth;
use crate::handlers::html::{ErrorResponse, make_error};
use crate::i18n::Lang;
use crate::{Database, Page, handlers::extract::Theme};
use wastebin_core::db::read::ListEntry;

#[derive(Template, WebTemplate)]
#[template(path = "admin.html")]
pub(crate) struct Admin {
    pub(crate) page: Page,
    pub(crate) theme: Option<Theme>,
    pub(crate) lang: Lang,
    pub(crate) entries: Vec<ListEntry>,
}

pub(crate) async fn get(
    _auth: AdminAuth,
    State(page): State<Page>,
    State(db): State<Database>,
    theme: Option<Theme>,
    lang: Lang,
) -> Result<Response, ErrorResponse> {
    async {
        let entries = db.list().await?;
        let admin = Admin {
            page: page.clone(),
            theme: theme.clone(),
            lang,
            entries,
        };
        Ok(admin.into_response())
    }
    .await
    .map_err(|err| make_error(err, page, theme, lang))
}

pub(crate) async fn delete(
    _auth: AdminAuth,
    Path(id): Path<String>,
    State(db): State<Database>,
    State(page): State<Page>,
    theme: Option<Theme>,
    lang: Lang,
) -> Result<Redirect, ErrorResponse> {
    async {
        let id_parsed = id.parse()?;
        db.delete_many(vec![id_parsed]).await?;
        Ok(Redirect::to("/admin"))
    }
    .await
    .map_err(|err| make_error(err, page, theme, lang))
}

#[derive(serde::Deserialize)]
pub(crate) struct UpdateForm {
    pub visibility: Option<String>,
    pub password: Option<String>,
    pub expires: Option<String>,
}

pub(crate) async fn update(
    _auth: AdminAuth,
    Path(id): Path<String>,
    State(db): State<Database>,
    State(page): State<Page>,
    theme: Option<Theme>,
    lang: Lang,
    Form(form): Form<UpdateForm>,
) -> Result<Redirect, ErrorResponse> {
    async {
        let id_parsed = id.parse()?;

        let is_private = form.visibility.and_then(|v| match v.as_str() {
            "private" => Some(true),
            "public" => Some(false),
            _ => None,
        });

        let password = form.password.filter(|p| !p.is_empty());

        let expires = if let Some(exp_str) = form.expires {
            if exp_str == "0" {
                Some(None)
            } else if let Ok(secs) = exp_str.parse::<u32>() {
                std::num::NonZeroU32::new(secs).map(|v| Some(v))
            } else {
                None
            }
        } else {
            None
        };

        db.update(id_parsed, is_private, expires, password).await?;
        Ok(Redirect::to("/admin"))
    }
    .await
    .map_err(|err| make_error(err, page, theme, lang))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::{Client, StoreCookies};
    use crate::handlers::insert::form::Entry;
    use reqwest::StatusCode;

    #[tokio::test]
    async fn admin_auth_failed() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;
        let res = client.get("/admin").send().await?;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        let res = client
            .get("/admin")
            .basic_auth("wrong", Some("wrong"))
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn admin_auth_success() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;

        let res = client
            .get("/admin")
            .basic_auth("admin", Some("admin"))
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::OK);

        let body = res.text().await?;
        assert!(body.contains("Admin Control Panel"));

        let data = Entry {
            text: String::from("admin test paste"),
            ..Default::default()
        };
        let post_res = client.post_form().form(&data).send().await?;
        assert_eq!(post_res.status(), StatusCode::SEE_OTHER);
        let location = post_res.headers().get("location").unwrap().to_str()?.to_owned();
        let id = location.split('/').last().unwrap();

        let res = client
            .get("/admin")
            .basic_auth("admin", Some("admin"))
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.text().await?;
        assert!(body.contains(id));

        let delete_res = client
            .post(&format!("/admin/delete/{id}"))
            .basic_auth("admin", Some("admin"))
            .send()
            .await?;
        assert_eq!(delete_res.status(), StatusCode::SEE_OTHER);

        let check_res = client.get(&format!("/p/{id}")).send().await?;
        assert_eq!(check_res.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn custom_padawan_error() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;

        let res = client.get("/p/nonexistent").send().await?;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body = res.text().await?;
        assert!(body.contains("I felt a great disturbance in the Force, as if an error occurred and this page vanished. Check your path, young padawan."));

        let res = client.get("/p/invalid_id_length_more_than_six_chars").send().await?;
        assert!(res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::BAD_REQUEST);
        let body = res.text().await?;
        assert!(body.contains("I felt a great disturbance in the Force, as if an error occurred and this page vanished. Check your path, young padawan."));

        Ok(())
    }

    #[tokio::test]
    async fn admin_update_paste() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new(StoreCookies(false)).await;

        let data = Entry {
            text: String::from("admin update paste test"),
            ..Default::default()
        };
        let post_res = client.post_form().form(&data).send().await?;
        assert_eq!(post_res.status(), StatusCode::SEE_OTHER);
        let location = post_res.headers().get("location").unwrap().to_str()?.to_owned();
        let id = location.split('/').last().unwrap();

        let mut form_data = std::collections::HashMap::new();
        form_data.insert("visibility", "private");
        let res = client
            .post(&format!("/admin/update/{id}"))
            .basic_auth("admin", Some("admin"))
            .form(&form_data)
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let res = client.get(&format!("/s/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.text().await?;
        assert!(body.contains("admin update paste test"));

        let mut form_data = std::collections::HashMap::new();
        form_data.insert("password", "new_admin_pass");
        let res = client
            .post(&format!("/admin/update/{id}"))
            .basic_auth("admin", Some("admin"))
            .form(&form_data)
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let res = client.get(&format!("/s/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.text().await?;
        assert!(body.contains("password") || body.contains("type=\"password\""));

        let mut form_data = std::collections::HashMap::new();
        form_data.insert("expires", "600");
        let res = client
            .post(&format!("/admin/update/{id}"))
            .basic_auth("admin", Some("admin"))
            .form(&form_data)
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        Ok(())
    }
}

