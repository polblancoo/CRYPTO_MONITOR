use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use crate::{Database, User};
use argon2::password_hash::Error as ArgonError;

#[derive(Debug)]
pub enum AuthError {
    DatabaseError(tokio_rusqlite::Error),
    UserExists,
    InvalidCredentials,
    HashError(String),
}

// Implementar Send y Sync para AuthError
unsafe impl Send for AuthError {}
unsafe impl Sync for AuthError {}

impl std::error::Error for AuthError {}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::DatabaseError(e) => write!(f, "Error de base de datos: {}", e),
            AuthError::UserExists => write!(f, "El usuario ya existe"),
            AuthError::InvalidCredentials => write!(f, "Credenciales inválidas"),
            AuthError::HashError(e) => write!(f, "Error al hashear contraseña: {}", e),
        }
    }
}

// Implementar From para los errores de Argon2
impl From<ArgonError> for AuthError {
    fn from(err: ArgonError) -> Self {
        AuthError::HashError(err.to_string())
    }
}

pub struct Auth<'a> {
    db: &'a Database,
}

impl<'a> Auth<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub async fn register_user(&self, username: &str, password: &str) -> Result<User, AuthError> {
        let password_hash = self.hash_password(password)?;
        
        let user_id = self.db.create_user(username.to_string(), password_hash.clone())
            .await
            .map_err(|e| AuthError::DatabaseError(e))?;
        
        Ok(User {
            id: user_id,
            username: username.to_string(),
            password_hash,
            api_key: None,
            telegram_chat_id: None,
            created_at: chrono::Utc::now().timestamp(),
            last_login: None,
            is_active: true,
        })
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<Option<User>, AuthError> {
        let user = self.db.get_user_by_username(username)
            .await
            .map_err(|e| AuthError::DatabaseError(e))?;

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