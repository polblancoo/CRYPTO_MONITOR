use std::sync::Arc;
use tokio::{self, signal, task::JoinSet, sync::broadcast};
use dotenv::dotenv;
use crypto_monitor::{
    db::Database,
    bot::TelegramBot,
    monitor::PriceMonitor,
    exchanges::ExchangeManager,
    crypto_api::CryptoAPI,
    notify::NotificationService,
};
use teloxide::Bot;
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter};
use std::time::Duration;

#[tokio::main(worker_threads = 4)]
async fn main() {
    // Cargar variables de entorno
    dotenv().ok();
    
    // Inicializar logging con más detalle
    fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Iniciando Crypto Monitor...");

    // Inicializar base de datos de manera asíncrona
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:crypto_monitor.db".to_string());

    let db = match Database::new(&database_url).await {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Error al inicializar la base de datos: {}", e);
            return;
        }
    };

    // Crear el exchange manager
    let exchange_manager = Arc::new(
        ExchangeManager::new()
            .expect("Error al inicializar el exchange manager")
    );

    // Obtener token de Telegram
    let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN debe estar configurado");
    let bot = Bot::new(telegram_token);

    // Inicializar servicios necesarios
    let api = CryptoAPI::new(
        std::env::var("COINGECKO_API_KEY")
            .unwrap_or_default()
    );

    let notification_service = NotificationService::new(
        std::env::var("TELEGRAM_BOT_TOKEN")
            .expect("TELEGRAM_BOT_TOKEN debe estar configurado")
    );

    // Inicializar el monitor de precios
    let check_interval = std::env::var("CHECK_INTERVAL")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<u64>()
        .unwrap_or(60);

    let monitor = PriceMonitor::new(
        api,
        notification_service,
        db.clone(),
        check_interval,
    );

    // Inicializar el bot de Telegram con reintento automático
    let telegram_bot = Arc::new(TelegramBot::new(
        db.clone(), 
        exchange_manager.clone(),
        bot.clone()
    ));

    info!("Iniciando servicios...");

    // Crear un canal para señalización de cierre
    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let mut tasks = JoinSet::new();

    // Spawn del bot con reintento
    {
        let mut rx = shutdown_tx.subscribe();
        let bot = Arc::clone(&telegram_bot);
        tasks.spawn(async move {
            loop {
                tokio::select! {
                    _ = bot.run() => {
                        error!("Bot detenido, reintentando en 5 segundos...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    _ = rx.recv() => {
                        info!("Bot recibió señal de cierre");
                        break;
                    }
                }
            }
        });
    }

    // Spawn del monitor
    {
        let mut rx = shutdown_tx.subscribe();
        tasks.spawn(async move {
            tokio::select! {
                result = monitor.start() => {
                    if let Err(e) = result {
                        error!("Error en el monitor: {}", e);
                    }
                }
                _ = rx.recv() => info!("Monitor recibió señal de cierre"),
            }
        });
    }

    // Esperar señal de cierre
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Recibida señal de interrupción");
        }
    }

    // Cerrar todo ordenadamente
    info!("Iniciando cierre ordenado...");
    
    // Notificar a todas las tareas
    let _ = shutdown_tx.send(());
    
    // Esperar a que todas las tareas terminen
    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            error!("Error al cerrar tarea: {}", e);
        }
    }

    info!("Servicios detenidos correctamente");
} 