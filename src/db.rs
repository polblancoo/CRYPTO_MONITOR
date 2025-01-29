use rusqlite::{Connection, params, Result as SqliteResult, OptionalExtension};
use std::error::Error;
use std::fs;
use std::path::Path;
use chrono::Utc;
use crate::models::{User, PriceAlert, ApiKey, AlertType, UserState};
use std::sync::Mutex;
use tracing::info;
use serde_json;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Remover el prefijo "sqlite:" y obtener la ruta absoluta
        let db_path = database_url.trim_start_matches("sqlite:");
        
        // Convertir a ruta absoluta
        let absolute_path = if Path::new(db_path).is_relative() {
            std::env::current_dir()?.join(db_path)
        } else {
            Path::new(db_path).to_path_buf()
        };

        println!("Ruta absoluta de la base de datos: {}", absolute_path.display());

        // Asegurar que el directorio padre existe
        if let Some(parent) = absolute_path.parent() {
            println!("Creando directorio: {}", parent.display());
            fs::create_dir_all(parent)?;
        }

        // Crear conexión
        let conn = Connection::open(&absolute_path)?;

        // Habilitar foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Crear las tablas
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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_states (
                chat_id INTEGER PRIMARY KEY,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Verificar que las tablas se crearon antes de mover conn
        let table_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'price_alerts', 'api_keys', 'user_states')",
            [],
            |row| row.get(0),
        )?;

        if table_count != 4 {
            return Err("No se pudieron crear todas las tablas".into());
        }

        // Crear instancia de Database
        let db = Self { conn: Mutex::new(conn) };
        
        // Migrar datos existentes si es necesario
        Database::migrate_alerts_table(&db.conn.lock().unwrap())?;

        println!("Tablas creadas correctamente");
        Ok(db)
    }

    fn migrate_alerts_table(conn: &Connection) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Verificar si necesitamos migrar
        let needs_migration = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('price_alerts') WHERE name = 'target_price'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0) > 0;

        if needs_migration {
            info!("Iniciando migración de la tabla price_alerts");
            
            // Crear tabla temporal con nueva estructura
            conn.execute(
                "CREATE TABLE price_alerts_new (
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

            // Migrar datos existentes
            conn.execute(
                r#"
                INSERT INTO price_alerts_new (id, user_id, symbol, alert_type, created_at, triggered_at, is_active)
                SELECT 
                    id, 
                    user_id, 
                    symbol, 
                    json_object(
                        'type', 'Price',
                        'data', json_object(
                            'target_price', target_price,
                            'condition', condition
                        )
                    ) as alert_type,
                    created_at,
                    triggered_at,
                    is_active
                FROM price_alerts
                "#,
                [],
            )?;

            // Reemplazar tabla antigua con la nueva
            conn.execute("DROP TABLE price_alerts", [])?;
            conn.execute("ALTER TABLE price_alerts_new RENAME TO price_alerts", [])?;

            info!("Migración completada exitosamente");
        }

        Ok(())
    }

    pub fn create_user(&self, username: &str, password_hash: &str) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO users (username, password_hash, created_at) VALUES (?, ?, ?)",
            params![username, password_hash, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn verify_user(&self, username: &str, password_hash: &str) -> SqliteResult<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM users WHERE username = ? AND password_hash = ? AND is_active = 1")?;
        let mut rows = stmt.query_map(params![username, password_hash], |row| {
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
        })?;

        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn save_alert(&self, alert: &PriceAlert) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        // Serializar alert_type a JSON
        let alert_type_json = serde_json::to_string(&alert.alert_type)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        conn.execute(
            "INSERT INTO price_alerts (
                user_id, symbol, alert_type, created_at, is_active
            ) VALUES (?, ?, ?, ?, ?)",
            params![
                alert.user_id,
                alert.symbol,
                alert_type_json,
                now,
                true
            ],
        )?;
        Ok(())
    }

    pub fn get_active_alerts(&self) -> SqliteResult<Vec<PriceAlert>> {
        let conn = self.conn.lock().unwrap();
        info!("Consultando alertas activas de la base de datos");
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
        .collect::<SqliteResult<Vec<_>>>()?;

        info!("Encontradas {} alertas activas en la base de datos", alerts.len());
        Ok(alerts)
    }

    pub fn mark_alert_triggered(&self, alert_id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            "UPDATE price_alerts SET triggered_at = ?, is_active = 0 WHERE id = ?",
            params![now, alert_id],
        )?;
        Ok(())
    }

    pub fn create_api_key(&self, user_id: i64) -> SqliteResult<ApiKey> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        let key = generate_api_key();
        let expires_at = now + (30 * 24 * 60 * 60); // 30 días

        conn.execute(
            "INSERT INTO api_keys (user_id, key, created_at, expires_at, is_active) 
             VALUES (?, ?, ?, ?, ?)",
            params![user_id, key, now, expires_at, true],
        )?;

        let id: i64 = conn.last_insert_rowid();
        Ok(ApiKey {
            id,
            user_id,
            key,
            created_at: now,
            last_used: None,
            expires_at: Some(expires_at),
            is_active: true,
        })
    }

    pub fn verify_api_key(&self, api_key: &str) -> SqliteResult<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        let mut stmt = conn.prepare(
            r#"
            SELECT u.* 
            FROM users u
            JOIN api_keys k ON u.id = k.user_id
            WHERE k.key = ? 
              AND k.is_active = 1 
              AND u.is_active = 1
              AND (k.expires_at IS NULL OR k.expires_at > ?)
            "#
        )?;
        
        let mut rows = stmt.query_map(params![api_key, now], |row| {
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
        })?;

        match rows.next() {
            Some(result) => {
                let user = result?;
                // Actualizar last_used
                conn.execute(
                    "UPDATE api_keys SET last_used = ? WHERE key = ?",
                    params![now, api_key],
                )?;
                Ok(Some(user))
            },
            None => Ok(None),
        }
    }

    pub fn get_user_alerts(&self, user_id: i64) -> SqliteResult<Vec<PriceAlert>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                id as 'id?',
                user_id,
                symbol,
                alert_type,
                created_at,
                triggered_at,
                is_active
            FROM price_alerts 
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#
        )?;
        let alerts_iter = stmt.query_map(params![user_id], |row| {
            let alert_type_json: String = row.get(3)?;
            let alert_type: AlertType = serde_json::from_str(&alert_type_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            Ok(PriceAlert {
                id: row.get(0)?,
                user_id: row.get(1)?,
                symbol: row.get(2)?,
                alert_type,
                created_at: row.get(4)?,
                triggered_at: row.get(5)?,
                is_active: row.get(6)?,
            })
        })?;

        let mut alerts = Vec::new();
        for alert in alerts_iter {
            alerts.push(alert?);
        }
        Ok(alerts)
    }

    pub fn get_user_by_username(&self, username: &str) -> SqliteResult<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM users WHERE username = ? AND is_active = 1")?;
        let mut rows = stmt.query_map(params![username], |row| {
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
        })?;

        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn get_alert(&self, alert_id: i64) -> SqliteResult<Option<PriceAlert>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, symbol, alert_type, created_at, triggered_at, is_active 
             FROM price_alerts 
             WHERE id = ?"
        )?;

        let mut rows = stmt.query_map([alert_id], |row| {
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
        })?;

        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn delete_alert(&self, alert_id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM price_alerts WHERE id = ?",
            params![alert_id],
        )?;
        Ok(())
    }

    pub fn update_user_telegram_chat_id(&self, user_id: i64, chat_id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE users SET telegram_chat_id = ? WHERE id = ?",
            params![chat_id, user_id],
        )?;
        Ok(())
    }

    pub fn get_user_api_key(&self, user_id: i64) -> SqliteResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT key FROM api_keys 
             WHERE user_id = ? AND is_active = 1 
             ORDER BY created_at DESC LIMIT 1"
        )?;
        
        let mut rows = stmt.query_map([user_id], |row| row.get(0))?;
        
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn get_user_telegram_chat_id(&self, user_id: i64) -> SqliteResult<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT telegram_chat_id FROM users WHERE id = ?",
            [user_id],
            |row| row.get(0)
        ).optional()
    }

    pub fn get_user_by_telegram_id(&self, chat_id: i64) -> SqliteResult<Option<User>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT * FROM users WHERE telegram_chat_id = ?",
            [chat_id],
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
            }
        ).optional()
    }

    pub fn save_user_state(&self, chat_id: i64, state: &UserState) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        let state_json = serde_json::to_string(state)
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
    }

    pub fn get_user_state(&self, chat_id: i64) -> SqliteResult<Option<UserState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT state FROM user_states WHERE chat_id = ?"
        )?;

        let state = stmt.query_row([chat_id], |row| {
            let state_json: String = row.get(0)?;
            let state: UserState = serde_json::from_str(&state_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            Ok(state)
        }).optional()?;

        Ok(state)
    }

    pub fn clear_user_state(&self, chat_id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM user_states WHERE chat_id = ?",
            [chat_id]
        )?;
        Ok(())
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