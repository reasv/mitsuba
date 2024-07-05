use std::pin::Pin;
use actix_session::SessionExt;
use actix_web::{Error, FromRequest, web};
use futures::Future;
use log::error;
use std::marker::PhantomData;

use crate::{archiver::Archiver, models::{UserRole, User}};

pub struct AuthUser<R: RoleCheck = AnyRole>{
    pub name: String,
    pub role: UserRole,
    _marker: PhantomData<R>
}

impl<R: RoleCheck> AuthUser<R> {
    pub fn anonymous() -> AuthUser<R> {
        AuthUser {
            name: "Anonymous".to_string(),
            role: UserRole::Anonymous,
            _marker: PhantomData
        }
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
impl<R: RoleCheck> From<User> for AuthUser<R> {
    fn from(user: User) -> Self {
        AuthUser {
            name: user.name,
            role: user.role,
            _marker: PhantomData
        }
    }
}

impl<R: RoleCheck> FromRequest for AuthUser<R> {
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
                Ok(AuthUser::<R>::anonymous())
            } else {
                Err(actix_web::error::ErrorForbidden("User Must be authenticated"))
            };
            match (archiver_data, username_opt) {
                (Some(data), Some(username)) => {
                    let archiver = data.get_ref();
                    if let Some(user_struct) = archiver.db_client
                        .get_user(&username).await
                        .map_err(|e| error!("Could not retrieve session user data from db (user: {}): {}", username, e))
                        .ok()
                        .flatten() {
                            let auth_user: AuthUser<R> = user_struct.into();
                            if R::is_allowed(&auth_user.role) {
                                return Ok(auth_user)
                            } else {
                                return Err(actix_web::error::ErrorForbidden("User role not allowed"));
                            }
                        }
                    anonymous_result
                },
                _ => anonymous_result
            }
        })
    }
}
