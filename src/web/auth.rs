use std::pin::Pin;
use actix_session::SessionExt;
use actix_web::{Error, FromRequest, web, ResponseError, HttpResponse};
use futures::Future;
use log::error;
use serde::Serialize;
use std::marker::PhantomData;

use crate::{archiver::Archiver, models::{UserRole, User}};

pub struct AuthUser<R: RoleCheck = AnyRole, E: RoleCheckError = JSONRCError>{
    pub name: String,
    pub role: UserRole,
    _marker: PhantomData<R>,
    _error_marker: PhantomData<E>
}

impl<R: RoleCheck, E: RoleCheckError> AuthUser<R, E> {
    pub fn anonymous() -> AuthUser<R, E> {
        AuthUser {
            name: "Anonymous".to_string(),
            role: UserRole::Anonymous,
            _marker: PhantomData,
            _error_marker: PhantomData
        }
    }
}

pub trait RoleCheckError {
    fn not_authenticated() -> actix_web::Error;
    fn not_authorized(role: &UserRole) -> actix_web::Error;
}

pub struct JSONRCError;
pub struct TextRCError;

impl RoleCheckError for TextRCError {
    fn not_authenticated() -> actix_web::Error {
        actix_web::error::ErrorUnauthorized("User must be authenticated")
    }

    fn not_authorized(role: &UserRole) -> actix_web::Error {
        actix_web::error::ErrorForbidden(format!("User role not authorized for this action: {:?}", role))
    }
}

impl RoleCheckError for JSONRCError {
    fn not_authenticated() -> actix_web::Error {
        JSONError {
            error: "Not Authenticated".to_string(),
            code: actix_web::http::StatusCode::UNAUTHORIZED,
            message: "User must be authenticated".to_string()
        }.into()
    }

    fn not_authorized(role: &UserRole) -> actix_web::Error {
        JSONError {
            error: "Not Authorized".to_string(),
            code: actix_web::http::StatusCode::FORBIDDEN,
            message: format!("User role not authorized for this action: {:?}", role)
        }.into()
    }
}

#[derive(Debug, Serialize)]
struct JSONError {
    error: String,
    message: String,
    #[serde(skip)]
    code: actix_web::http::StatusCode
}

impl std::fmt::Display for JSONError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl ResponseError for JSONError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .json(self)
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        self.code
    }
}

pub struct AnyRole;

impl RoleCheck for AnyRole {
    fn is_allowed(_role: &UserRole) -> bool {
        true
    }
}

pub struct Authenticated;
pub struct AnonymousOnly;
pub struct AdminOnly;

impl RoleCheck for AdminOnly {
    fn is_allowed(role: &UserRole) -> bool {
        matches!(role, UserRole::Admin)
    }
}

impl RoleCheck for Authenticated {
    fn is_allowed(role: &UserRole) -> bool {
        match role {
            UserRole::Anonymous => false,
            _ => true,
        }
    }
}

impl RoleCheck for AnonymousOnly {
    fn is_allowed(role: &UserRole) -> bool {
        matches!(role, UserRole::Anonymous)
    }
}

pub trait RoleCheck {
    fn is_allowed(role: &UserRole) -> bool;
}

// implement from User for session user
impl<R: RoleCheck, E: RoleCheckError> From<User> for AuthUser<R, E> {
    fn from(user: User) -> Self {
        AuthUser {
            name: user.name,
            role: user.role,
            _marker: PhantomData,
            _error_marker: PhantomData
        }
    }
}

impl<R: RoleCheck, E: RoleCheckError> FromRequest for AuthUser<R, E> {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let session = req.get_session();
        let archiver_data = req.app_data::<web::Data::<Archiver>>()
            .map(|data| data.clone());
        let username_opt = session.get::<String>("username")
            .map_err(|e| error!("Could not retrieve session username: {}", e))
            .ok().flatten();
        Box::pin(async move {
            let anonymous_result = if R::is_allowed(&UserRole::Anonymous) {
                Ok(AuthUser::<R, E>::anonymous())
            } else {
                Err(E::not_authenticated())
            };
            match (archiver_data, username_opt) {
                (Some(data), Some(username)) => {
                    let archiver = data.get_ref();
                    if let Some(user_struct) = archiver.db_client
                        .get_user(&username).await
                        .map_err(|e| error!("Could not retrieve session user data from db (user: {}): {}", username, e))
                        .ok()
                        .flatten() {
                            let auth_user: AuthUser<R, E> = user_struct.into();
                            if R::is_allowed(&auth_user.role) {
                                return Ok(auth_user)
                            } else {
                                return Err(E::not_authorized(&auth_user.role));
                            }
                        }
                    anonymous_result
                },
                _ => anonymous_result
            }
        })
    }
}
