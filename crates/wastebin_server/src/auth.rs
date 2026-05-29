use axum::{
    extract::FromRequestParts,
    http::{
        header::WWW_AUTHENTICATE,
        request::Parts,
        StatusCode,
    },
    response::{IntoResponse, Response},
};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

pub(crate) struct AdminAuth;

impl<S> FromRequestParts<S> for AdminAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Ok(TypedHeader(Authorization(basic))) =
            TypedHeader::<Authorization<Basic>>::from_request_parts(parts, state).await
        {
            let expected_user = crate::env::admin_user();
            let expected_pass = crate::env::admin_password();

            if basic.username() == expected_user && basic.password() == expected_pass {
                return Ok(AdminAuth);
            }
        }

        let response = (
            StatusCode::UNAUTHORIZED,
            [(WWW_AUTHENTICATE, "Basic realm=\"Admin Panel\"")],
            "Unauthorized",
        )
            .into_response();

        Err(response)
    }
}
