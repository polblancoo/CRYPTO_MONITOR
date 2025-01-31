use rusqlite::{params, OptionalExtension};
use std::fs;
use std::path::Path;
use chrono::Utc;
use crate::{
    models::{User, PriceAlert, ApiKey, AlertType, UserState},
    exchanges::ExchangeCredentials,
};
use tracing::info;
use serde_json;
use tokio_rusqlite::Connection as AsyncConnection;
use std::sync::Arc;
use tokio_rusqlite::Error as AsyncSqliteError;

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
            
            // Crear las tablas necesarias
            conn.execute(
                "CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT UNIQUE NOT NULL,
                    password_hash TEXT NOT NULL,
                    api_key TEXT UNIQUE,
                    telegram_chat_id INTEGER,
                    created_at INTEGER NOT NULL,
                    last_login INTEGER,
                    is_active BOOLEAN NOT NULL DEFAULT 1
                )",
                [],
            )?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS price_alerts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL,
                    symbol TEXT NOT NULL,
                    alert_type TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    triggered_at INTEGER,
                    is_active BOOLEAN NOT NULL DEFAULT 1,
                    FOREIGN KEY(user_id) REFERENCES users(id)
                )",
                [],
            )?;

            // ... otras tablas necesarias ...

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

    pub async fn save_alert(&self, alert: PriceAlert) -> Result<(), AsyncSqliteError> {
        let alert_clone = alert;
        self.conn.call(move |conn| {
            let now = Utc::now().timestamp();
            let alert_type_json = serde_json::to_string(&alert_clone.alert_type)
                .map_err(|e| AsyncSqliteError::Other(Box::new(e)))?;

            conn.execute(
                "INSERT INTO price_alerts (user_id, symbol, alert_type, created_at) 
                 VALUES (?, ?, ?, ?)",
                params![
                    alert_clone.user_id,
                    alert_clone.symbol,
                    alert_type_json,
                    now,
                ],
            )?;
            Ok(())
        }).await
    }

    pub async fn get_active_alerts(&self) -> Result<Vec<PriceAlert>, AsyncSqliteError> {
        self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, symbol, alert_type, created_at, triggered_at, is_active 
                 FROM price_alerts 
                 WHERE is_active = 1 AND triggered_at IS NULL"
            )?;

            let alerts = stmt.query_map([], |row| {
                let alert_type_json: String = row.get(3)?;
                let alert_type: AlertType = serde_json::from_str(&alert_type_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                Ok(PriceAlert {
                    id: Some(row.get(0)?),
                    user_id: row.get(1)?,
                    symbol: row.get(2)?,
                    alert_type,
                    created_at: row.get(4)?,
                    triggered_at: row.get(5)?,
                    is_active: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

            Ok(alerts)
        }).await
    }

    pub async fn mark_alert_triggered(&self, alert_id: i64) -> Result<(), AsyncSqliteError> {
        self.conn.call(move |conn| {
            let now = Utc::now().timestamp();
            conn.execute(
                "UPDATE price_alerts SET triggered_at = ?, is_active = 0 WHERE id = ?",
                params![now, alert_id],
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
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, symbol, alert_type, created_at, triggered_at, is_active 
                 FROM price_alerts 
                 WHERE user_id = ?"
            )?;

            let alerts = stmt.query_map([user_id], |row| {
                let alert_type_json: String = row.get(3)?;
                let alert_type: AlertType = serde_json::from_str(&alert_type_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                Ok(PriceAlert {
                    id: Some(row.get(0)?),
                    user_id: row.get(1)?,
                    symbol: row.get(2)?,
                    alert_type,
                    created_at: row.get(4)?,
                    triggered_at: row.get(5)?,
                    is_active: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

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
                "SELECT id, user_id, symbol, alert_type, created_at, triggered_at, is_active 
                 FROM price_alerts 
                 WHERE id = ?"
            )?;

            let alert = stmt.query_row([alert_id], |row| {
                let alert_type_json: String = row.get(3)?;
                let alert_type: AlertType = serde_json::from_str(&alert_type_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                Ok(PriceAlert {
                    id: Some(row.get(0)?),
                    user_id: row.get(1)?,
                    symbol: row.get(2)?,
                    alert_type,
                    created_at: row.get(4)?,
                    triggered_at: row.get(5)?,
                    is_active: row.get(6)?,
                })
            }).optional()?;

            Ok(alert)
        }).await
    }

    pub async fn delete_alert(&self, alert_id: i64) -> Result<(), AsyncSqliteError> {
        self.conn.call(move |conn| {
            conn.execute(
                "DELETE FROM price_alerts WHERE id = ?",
                params![alert_id],
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