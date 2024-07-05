use std::pin::Pin;
use actix_session::SessionExt;
use actix_web::{Error, FromRequest, web};
use futures::Future;
use log::error;

use crate::{archiver::Archiver, models::{UserRole, User}};

pub struct AuthUser {
    pub name: String,
    pub role: UserRole
}

// implement from User for session user
impl From<User> for AuthUser {
    fn from(user: User) -> Self {
        AuthUser {
            name: user.name,
            role: user.role
        }
    }
}

impl FromRequest for AuthUser {
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
            match (archiver_data, username_opt) {
                (Some(data), Some(username)) => {
                    let archiver = data.get_ref();
                    if let Some(user_struct) = archiver.db_client
                        .get_user(&username).await
                        .map_err(|e| error!("Could not retrieve session user data from db (user: {}): {}", username, e))
                        .ok()
                        .flatten() {
                            return Ok(user_struct.into())
                        }
                    Ok(AuthUser {name: "Anonymous".to_string(), role: UserRole::Anonymous})
                },
                _ => Ok(AuthUser {name: "Anonymous".to_string(), role: UserRole::Anonymous})
            }
        })
    }
}
