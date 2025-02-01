mod commands;
mod handlers;

pub use commands::Command;
use handlers::*;

use teloxide::{
    prelude::*,
    dispatching::{UpdateHandler, HandlerExt, dialogue::InMemStorage},
    RequestError,
    ApiError,
};
use std::sync::Arc;
use tracing::{error, info};
use crate::{
    db::Database,
    exchanges::ExchangeManager,
    models::User,
};
use tokio::time::{sleep, Duration};
use reqwest;

const MAX_RETRIES: u32 = 10;
const INITIAL_RETRY_DELAY: u64 = 10;
const MAX_RETRY_DELAY: u64 = 300;

pub struct TelegramBot {
    bot: Bot,
    handler: UpdateHandler<RequestError>,
    db: Arc<Database>,
    exchange_manager: Arc<ExchangeManager>,
}

#[derive(Debug)]
pub enum BotError {
    Network(reqwest::Error),
    Telegram(RequestError),
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotError::Network(e) => write!(f, "Error de red: {}", e),
            BotError::Telegram(e) => write!(f, "Error de Telegram: {}", e),
            BotError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for BotError {}

impl From<reqwest::Error> for BotError {
    fn from(err: reqwest::Error) -> Self {
        BotError::Network(err)
    }
}

impl From<RequestError> for BotError {
    fn from(err: RequestError) -> Self {
        BotError::Telegram(err)
    }
}

impl TelegramBot {
    pub fn new(token: String, db: Arc<Database>, exchange_manager: Arc<ExchangeManager>) -> Self {
        let mut client_builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .tcp_nodelay(true);

        // Configurar proxy si está definido
        if let Ok(proxy_url) = std::env::var("HTTPS_PROXY") {
            if let Ok(proxy) = reqwest::Proxy::https(&proxy_url) {
                client_builder = client_builder.proxy(proxy);
                info!("Usando proxy HTTPS: {}", proxy_url);
            }
        }

        // Configurar proxy SOCKS si está definido
        if let Ok(proxy_url) = std::env::var("SOCKS_PROXY") {
            if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                client_builder = client_builder.proxy(proxy);
                info!("Usando proxy SOCKS: {}", proxy_url);
            }
        }

        let client = client_builder
            .build()
            .expect("Error al crear cliente HTTP");

        let bot = Bot::with_client(token, client);
        
        // Configurar comandos del bot con reintentos
        Command::set_my_commands(bot.clone());

        let db_clone = db.clone();
        let exchange_manager_clone1 = exchange_manager.clone();
        let exchange_manager_clone2 = exchange_manager.clone();
        
        let handler = dptree::entry()
            .branch(
                Update::filter_message()
                    .filter_command::<Command>()
                    .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                        let db = db_clone.clone();
                        let exchange_manager = exchange_manager_clone1.clone();
                        async move {
                            let result = match cmd {
                                Command::Help => handle_help(bot, msg).await,
                                Command::Start => handle_start(bot, msg, db).await,
                                Command::Register { text } => handle_register(bot, msg, db.clone(), text).await,
                                Command::Alert { text } => handle_alert(bot, msg, text, db.clone()).await,
                                Command::Depeg { text } => handle_depeg(bot, msg, text).await,
                                Command::PairDepeg { text } => handle_pair_depeg(bot, msg, text).await,
                                Command::Alerts => handle_list_alerts(bot, msg, db.clone()).await,
                                Command::Delete { text } => handle_delete_alert(bot, msg, text).await,
                                Command::Symbols => handle_symbols(bot, msg).await,
                                Command::Balance { text: _ } => handle_balance(bot, msg, exchange_manager).await,
                                Command::Connect { text } => {
                                    let args = text.split_whitespace().map(String::from).collect();
                                    handle_connect(bot, msg, db, args).await
                                },
                                Command::Buy { text } => handle_buy(bot, msg, db.clone(), exchange_manager.clone(), text).await,
                                Command::Sell { text } => handle_sell(bot, msg, db.clone(), exchange_manager.clone(), text).await,
                                Command::Orders { text: _ } => handle_orders(bot, msg, exchange_manager).await,
                                Command::Order(text) => handle_order(bot, msg, text, exchange_manager).await,
                                Command::Cancel { text } => handle_cancel(bot, msg, text, exchange_manager).await,
                            };
                            result.map_err(|e| RequestError::Api(ApiError::Unknown(e.to_string())))
                        }
                    })
            )
            .branch(
                Update::filter_callback_query()
                    .endpoint(move |bot: Bot, q: CallbackQuery| {
                        let exchange_manager = exchange_manager_clone2.clone();
                        async move {
                            handle_callback_query(bot, q, exchange_manager).await
                                .map_err(|e| RequestError::Api(ApiError::Unknown(e.to_string())))
                        }
                    })
            );

        Self {
            bot,
            handler: handler.into(),
            db,
            exchange_manager,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bot = self.bot.clone();
        let db = self.db.clone();
        let exchange_manager = self.exchange_manager.clone();

        info!("Iniciando el bot...");
        
        match Dispatcher::builder(bot.clone(), self.handler.clone())
            .dependencies(dptree::deps![db.clone(), exchange_manager.clone(), InMemStorage::<()>::new()])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await
        {
            () => {
                info!("Bot detenido correctamente");
                Ok(())
            }
        }
    }

    pub async fn get_user_by_chat_id(&self, chat_id: i64) -> Result<User, Box<dyn std::error::Error + Send + Sync>> {
        match self.db.get_user_by_telegram_id(chat_id).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => Err("Usuario no encontrado".into()),
            Err(e) => Err(e.into())
        }
    }
} 