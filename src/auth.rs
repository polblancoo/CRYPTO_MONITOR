use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString, Error as ArgonError},
    Argon2,
};
use crate::{Database, User};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AuthError {
    ArgonError(ArgonError),
    DatabaseError(Box<dyn Error>),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::ArgonError(e) => write!(f, "Password hashing error: {}", e),
            AuthError::DatabaseError(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl Error for AuthError {}

impl From<ArgonError> for AuthError {
    fn from(err: ArgonError) -> Self {
        AuthError::ArgonError(err)
    }
}

pub struct Auth<'a> {
    db: &'a Database,
}

impl<'a> Auth<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn register_user(&self, username: &str, password: &str) -> Result<User, AuthError> {
        let password_hash = self.hash_password(password)?;
        let user_id = self.db.create_user(username, &password_hash)
            .map_err(|e| AuthError::DatabaseError(Box::new(e)))?;
        
        Ok(User {
            id: user_id,
            username: username.to_string(),
            password_hash,
            api_key: None,
            created_at: chrono::Utc::now().timestamp(),
            last_login: None,
            is_active: true,
        })
    }

    pub fn login(&self, username: &str, password: &str) -> Result<Option<User>, AuthError> {
        let user = self.db.get_user_by_username(username)
            .map_err(|e| AuthError::DatabaseError(Box::new(e)))?;

        if let Some(user) = user {
            if self.verify_password(password, &user.password_hash)? {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }

    fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(password_hash.to_string())
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
} 