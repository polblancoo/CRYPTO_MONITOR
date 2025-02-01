use crate::models::{User, PriceAlert, ApiKey, UserState, AlertCondition};
use crate::exchanges::types::ExchangeCredentials;
use rusqlite::{params, OptionalExtension};
use std::fs;
use std::path::Path;
use chrono::Utc;
use tracing::info;
use serde_json;
use tokio_rusqlite::Connection as AsyncConnection;
use std::sync::Arc;
use tokio_rusqlite::Error as AsyncSqliteError;
use rusqlite::ToSql;

#[derive(Debug)]
pub enum DatabaseError {
    ForeignKeyViolation(String),
    SqliteError(rusqlite::Error),
    Other(String),
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        DatabaseError::SqliteError(err)
    }
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ForeignKeyViolation(msg) => write!(f, "Foreign key violation: {}", msg),
            DatabaseError::SqliteError(err) => write!(f, "SQLite error: {}", err),
            DatabaseError::Other(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<DatabaseError> for tokio_rusqlite::Error {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::ForeignKeyViolation(msg) => {
                tokio_rusqlite::Error::Rusqlite(rusqlite::Error::InvalidParameterName(msg))
            }
            DatabaseError::SqliteError(err) => {
                tokio_rusqlite::Error::Rusqlite(err)
            }
            DatabaseError::Other(msg) => {
                tokio_rusqlite::Error::Rusqlite(rusqlite::Error::InvalidParameterName(msg))
            }
        }
    }
}

pub struct Database {
    conn: Arc<AsyncConnection>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Remover el prefijo "sqlite:" y obtener la ruta absoluta
        let db_path = database_url.trim_start_matches("sqlite:");
        
        // Convertir a ruta absoluta
        let absolute_path = if Path::new(db_path).is_relative() {
            std::env::current_dir()?.join(db_path)
        } else {
            Path::new(db_path).to_path_buf()
        };

        info!("Ruta absoluta de la base de datos: {}", absolute_path.display());

        // Asegurar que el directorio padre existe
        if let Some(parent) = absolute_path.parent() {
            info!("Creando directorio: {}", parent.display());
            fs::create_dir_all(parent)?;
        }

        // Crear conexión asíncrona
        let conn = AsyncConnection::open(absolute_path).await?;

        // Habilitar foreign keys y crear tablas
        conn.call(|conn| {
            conn.execute("PRAGMA foreign_keys = ON", [])?;
            
            // Crear tabla de usuarios primero
            conn.execute(
                "CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT UNIQUE NOT NULL,
                    password_hash TEXT NOT NULL,
                    api_key TEXT UNIQUE,
                    telegram_chat_id INTEGER UNIQUE,
                    created_at INTEGER NOT NULL,
                    last_login INTEGER,
                    is_active BOOLEAN NOT NULL DEFAULT 1
                )",
                [],
            )?;

            // Crear tabla de api_keys
            conn.execute(
                "CREATE TABLE IF NOT EXISTS api_keys (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL,
                    key TEXT UNIQUE NOT NULL,
                    created_at INTEGER NOT NULL,
                    last_used INTEGER,
                    expires_at INTEGER,
                    is_active BOOLEAN NOT NULL DEFAULT 1,
                    FOREIGN KEY(user_id) REFERENCES users(id)
                )",
                [],
            )?;

            // Crear tabla de estados de usuario
            conn.execute(
                "CREATE TABLE IF NOT EXISTS user_states (
                    chat_id INTEGER PRIMARY KEY,
                    state TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                )",
                [],
            )?;

            // Recrear la tabla de alertas con el tipo correcto para condition
            conn.execute("DROP TABLE IF EXISTS alerts", [])?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS alerts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL,
                    symbol TEXT NOT NULL,
                    target_price REAL NOT NULL,
                    condition TEXT NOT NULL CHECK(condition IN ('ABOVE', 'BELOW')),
                    created_at INTEGER NOT NULL,
                    triggered BOOLEAN NOT NULL DEFAULT 0,
                    FOREIGN KEY(user_id) REFERENCES users(id)
                )",
                [],
            )?;

            Ok(())
        }).await?;

        Ok(Self {
            conn: Arc::new(conn),
        })
    }

    // Implementar los métodos necesarios para usuarios y alertas
    pub async fn get_user_by_telegram_id(&self, telegram_id: i64) -> Result<Option<User>, AsyncSqliteError> {
        self.conn.call(move |conn| {
            Ok(conn.query_row(
                "SELECT * FROM users WHERE telegram_chat_id = ?",
                [telegram_id],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        password_hash: row.get(2)?,
                        api_key: row.get(3)?,
                        telegram_chat_id: row.get(4)?,
                        created_at: row.get(5)?,
                        last_login: row.get(6)?,
                        is_active: row.get(7)?,
                    })
                },
            ).optional()?)
        }).await
    }

    pub async fn create_user(&self, username: String, password_hash: String) -> Result<i64, AsyncSqliteError> {
        self.conn.call(move |conn| {
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT INTO users (username, password_hash, created_at) VALUES (?, ?, ?)",
                params![username, password_hash, now]
            )?;
            Ok(conn.last_insert_rowid())
        }).await
    }

    pub async fn save_alert(&self, alert: PriceAlert) -> Result<i64, AsyncSqliteError> {
        let alert_clone = alert.clone();
        self.conn.call(move |conn| {
            tracing::info!("Intentando guardar alerta para usuario {}", alert_clone.user_id);
            
            // Verificar que el usuario existe
            let user_exists: bool = conn.query_row(
                "SELECT 1 FROM users WHERE id = ?",
                [alert_clone.user_id],
                |_| Ok(true)
            ).unwrap_or(false);

            if !user_exists {
                tracing::error!("Usuario {} no encontrado", alert_clone.user_id);
                return Err(DatabaseError::ForeignKeyViolation(
                    format!("User id {} does not exist", alert_clone.user_id)
                ).into());
            }

            tracing::info!("Usuario existe, insertando alerta");
            let mut stmt = conn.prepare(
                "INSERT INTO alerts (user_id, symbol, target_price, condition, created_at, triggered)
                 VALUES (?, ?, ?, ?, ?, ?)"
            )?;

            let condition_str = alert_clone.condition.to_string().to_uppercase();
            tracing::info!(
                "Guardando alerta: symbol={}, price={}, condition={}, user_id={}",
                alert_clone.symbol,
                alert_clone.target_price,
                condition_str,
                alert_clone.user_id
            );

            let id = stmt.insert(params![
                alert_clone.user_id,
                alert_clone.symbol,
                alert_clone.target_price,
                condition_str,
                alert_clone.created_at,
                alert_clone.triggered,
            ])?;

            tracing::info!("Alerta insertada con id {}", id);
            Ok(id)
        }).await
    }

    pub async fn get_active_alerts(&self) -> Result<Vec<PriceAlert>, AsyncSqliteError> {
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, symbol, target_price, condition, created_at, triggered 
                 FROM alerts 
                 WHERE triggered = 0"
            )?;

            let alerts = stmt.query_map([], |row| {
                Ok(PriceAlert {
                    id: Some(row.get(0)?),
                    user_id: row.get(1)?,
                    symbol: row.get(2)?,
                    target_price: row.get(3)?,
                    condition: row.get(4)?,
                    created_at: row.get(5)?,
                    triggered: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

            Ok(alerts)
        }).await
    }

    pub async fn mark_alert_triggered(&self, alert_id: i64) -> Result<(), AsyncSqliteError> {
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE alerts SET triggered = 1 WHERE id = ?",
                [alert_id]
            )?;
            Ok(())
        }).await
    }

    pub async fn verify_api_key(&self, api_key: &str) -> Result<Option<User>, AsyncSqliteError> {
        let api_key = api_key.to_string();
        self.conn.call(move |conn| {
            Ok(conn.query_row(
                "SELECT u.* FROM users u 
                 INNER JOIN api_keys k ON u.id = k.user_id 
                 WHERE k.key = ? AND k.is_active = 1 
                 AND u.is_active = 1",
                [api_key],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        password_hash: row.get(2)?,
                        api_key: row.get(3)?,
                        telegram_chat_id: row.get(4)?,
                        created_at: row.get(5)?,
                        last_login: row.get(6)?,
                        is_active: row.get(7)?
                    })
                }
            ).optional()?)
        }).await
    }

    pub async fn save_exchange_credentials(
        &self,
        user_id: i64,
        exchange: &str,
        credentials: &ExchangeCredentials,
    ) -> Result<(), AsyncSqliteError> {
        let exchange = exchange.to_string();
        let credentials = credentials.clone();
        
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO exchange_credentials (user_id, exchange, api_key, api_secret) 
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    user_id,
                    exchange,
                    credentials.api_key,
                    credentials.api_secret,
                ],
            )?;
            Ok(())
        }).await
    }

    pub async fn get_user_alerts(&self, user_id: i64) -> Result<Vec<PriceAlert>, AsyncSqliteError> {
        let user_id = user_id;
        self.conn.call(move |conn| {
            tracing::info!("Buscando alertas para usuario {}", user_id);
            
            // Primero verificar si hay alertas
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM alerts WHERE user_id = ? AND triggered = 0",
                [user_id],
                |row| row.get(0)
            )?;
            
            tracing::info!("Encontradas {} alertas sin procesar", count);

            let mut stmt = conn.prepare(
                "SELECT id, user_id, symbol, target_price, condition, created_at, triggered 
                 FROM alerts 
                 WHERE user_id = ? AND triggered = 0
                 ORDER BY created_at DESC"
            )?;

            let alerts = stmt.query_map([user_id], |row| {
                let id: i64 = row.get(0)?;
                let user_id: i64 = row.get(1)?;
                let symbol: String = row.get(2)?;
                let target_price: f64 = row.get(3)?;
                let condition_str: String = row.get(4)?;
                let created_at: i64 = row.get(5)?;
                let triggered: bool = row.get(6)?;

                tracing::info!(
                    "Procesando alerta: id={}, user_id={}, symbol={}, price={}, condition={}, created={}, triggered={}",
                    id, user_id, symbol, target_price, condition_str, created_at, triggered
                );

                let condition = match condition_str.to_uppercase().as_str() {
                    "ABOVE" => AlertCondition::Above,
                    "BELOW" => AlertCondition::Below,
                    other => {
                        tracing::warn!("Condición inválida en la base de datos: {}", other);
                        AlertCondition::Above // valor por defecto
                    }
                };

                Ok(PriceAlert {
                    id: Some(id),
                    user_id,
                    symbol,
                    target_price,
                    condition,
                    created_at,
                    triggered,
                })
            })?
            .filter_map(|result| {
                if let Err(ref e) = result {
                    tracing::error!("Error al procesar alerta: {}", e);
                }
                result.ok()
            })
            .collect::<Vec<_>>();

            tracing::info!(
                "Recuperadas {} alertas activas para el usuario {}",
                alerts.len(),
                user_id
            );

            Ok(alerts)
        }).await
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, AsyncSqliteError> {
        let username = username.to_string();
        self.conn.call(move |conn| {
            Ok(conn.query_row(
                "SELECT * FROM users WHERE username = ? AND is_active = 1",
                [username],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        password_hash: row.get(2)?,
                        api_key: row.get(3)?,
                        telegram_chat_id: row.get(4)?,
                        created_at: row.get(5)?,
                        last_login: row.get(6)?,
                        is_active: row.get(7)?
                    })
                }
            ).optional()?)
        }).await
    }

    pub async fn update_user_telegram_chat_id(&self, user_id: i64, chat_id: i64) -> Result<(), AsyncSqliteError> {
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE users SET telegram_chat_id = ? WHERE id = ?",
                params![chat_id, user_id],
            )?;
            Ok(())
        }).await
    }

    pub async fn create_api_key(&self, user_id: i64) -> Result<ApiKey, AsyncSqliteError> {
        self.conn.call(move |conn| {
            let now = Utc::now().timestamp();
            let key = generate_api_key();
            let expires_at = now + (30 * 24 * 60 * 60); // 30 días

            conn.execute(
                "INSERT INTO api_keys (user_id, key, created_at, expires_at, is_active) 
                 VALUES (?, ?, ?, ?, ?)",
                params![user_id, key, now, expires_at, true],
            )?;

            let id = conn.last_insert_rowid();
            Ok(ApiKey {
                id,
                user_id,
                key,
                created_at: now,
                last_used: None,
                expires_at: Some(expires_at),
                is_active: true,
            })
        }).await
    }

    pub async fn get_alert(&self, alert_id: i64) -> Result<Option<PriceAlert>, AsyncSqliteError> {
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, symbol, target_price, condition, created_at, triggered 
                 FROM alerts 
                 WHERE id = ?"
            )?;

            let alert = stmt.query_row([alert_id], |row| {
                Ok(PriceAlert {
                    id: Some(row.get(0)?),
                    user_id: row.get(1)?,
                    symbol: row.get(2)?,
                    target_price: row.get(3)?,
                    condition: row.get(4)?,
                    created_at: row.get(5)?,
                    triggered: row.get(6)?,
                })
            }).optional()?;

            Ok(alert)
        }).await
    }

    pub async fn delete_alert(&self, alert_id: i64) -> Result<(), AsyncSqliteError> {
        self.conn.call(move |conn| {
            conn.execute(
                "DELETE FROM alerts WHERE id = ?",
                [alert_id],
            )?;
            Ok(())
        }).await
    }

    pub async fn save_user_state(&self, chat_id: i64, state: &UserState) -> Result<(), AsyncSqliteError> {
        let state = state.clone();
        self.conn.call(move |conn| {
            let now = Utc::now().timestamp();
            let state_json = serde_json::to_string(&state)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            conn.execute(
                "INSERT INTO user_states (chat_id, state, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)
                 ON CONFLICT(chat_id) DO UPDATE SET
                 state = ?2,
                 updated_at = ?3",
                params![chat_id, state_json, now],
            )?;

            Ok(())
        }).await
    }

    pub async fn get_user_state(&self, chat_id: i64) -> Result<Option<UserState>, AsyncSqliteError> {
        self.conn.call(move |conn| {
            let state = conn.query_row(
                "SELECT state FROM user_states WHERE chat_id = ?",
                [chat_id],
                |row| {
                    let state_json: String = row.get(0)?;
                    let state: UserState = serde_json::from_str(&state_json)
                        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                    Ok(state)
                }
            ).optional()?;
            
            Ok(state)
        }).await
    }

    pub async fn clear_user_state(&self, chat_id: i64) -> Result<(), AsyncSqliteError> {
        self.conn.call(move |conn| {
            conn.execute(
                "DELETE FROM user_states WHERE chat_id = ?",
                [chat_id]
            )?;
            Ok(())
        }).await
    }

    pub async fn get_user_api_key(&self, user_id: i64) -> Result<Option<String>, AsyncSqliteError> {
        self.conn.call(move |conn| {
            Ok(conn.query_row(
                "SELECT key FROM api_keys 
                 WHERE user_id = ? AND is_active = 1 
                 ORDER BY created_at DESC LIMIT 1",
                [user_id],
                |row| row.get(0)
            ).optional()?)
        }).await
    }

    pub async fn create_price_alert(&self, alert: PriceAlert) -> Result<i64, AsyncSqliteError> {
        let alert_clone = alert;
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "INSERT INTO alerts (user_id, symbol, target_price, condition, created_at, triggered) 
                 VALUES (?, ?, ?, ?, ?, ?)"
            )?;

            let id = stmt.insert([
                &alert_clone.user_id as &dyn ToSql,
                &alert_clone.symbol as &dyn ToSql,
                &alert_clone.target_price as &dyn ToSql,
                &alert_clone.condition.to_string() as &dyn ToSql,
                &alert_clone.created_at as &dyn ToSql,
                &alert_clone.triggered as &dyn ToSql,
            ])?;

            Ok(id)
        }).await
    }

    pub async fn create_telegram_user(&self, telegram_id: i64, username: &str) -> Result<i64, AsyncSqliteError> {
        let username = username.to_string();
        self.conn.call(move |conn| {
            let now = Utc::now().timestamp();
            
            // Insertar el usuario
            conn.execute(
                "INSERT INTO users (username, password_hash, telegram_chat_id, created_at, is_active) 
                 VALUES (?, '', ?, ?, 1)",
                params![username, telegram_id, now]
            )?;
            
            let user_id = conn.last_insert_rowid();
            Ok(user_id)
        }).await
    }
}

fn generate_api_key() -> String {
    use rand::{thread_rng, Rng};
    use rand::distributions::Alphanumeric;

    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
} 