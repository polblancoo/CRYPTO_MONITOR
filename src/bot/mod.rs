mod commands;
mod handlers;

pub use commands::Command;
use handlers::*;

use teloxide::{
    prelude::*,
    dispatching::{UpdateHandler, HandlerExt},
    RequestError,
    ApiError,
};
use std::sync::Arc;
use crate::{
    db::Database,
    exchanges::ExchangeManager,
    models::User,
};

pub struct TelegramBot {
    bot: Bot,
    handler: UpdateHandler<RequestError>,
    db: Arc<Database>,
    exchange_manager: Arc<ExchangeManager>,
}

impl TelegramBot {
    pub fn new(token: String, db: Arc<Database>, exchange_manager: Arc<ExchangeManager>) -> Self {
        let bot = Bot::new(token);
        let db_clone = db.clone();
        let exchange_manager_clone = exchange_manager.clone();
        
        let handler = Update::filter_message()
            .branch(
                dptree::entry()
                    .filter_command::<Command>()
                    .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                        let db = db_clone.clone();
                        let exchange_manager = exchange_manager_clone.clone();
                        async move {
                            match cmd {
                                Command::Help => handle_help(bot, msg).await,
                                Command::Start => handle_start(bot, msg, db).await,
                                Command::Register { text } => handle_register(bot, msg, db, text).await,
                                Command::Alert => handle_alert_creation(bot, msg).await,
                                Command::Depeg => handle_depeg(bot, msg).await,
                                Command::PairDepeg => handle_pair_depeg(bot, msg).await,
                                Command::Alerts => handle_list_alerts(bot, msg).await,
                                Command::Delete => handle_delete_alert(bot, msg).await,
                                Command::Symbols => handle_symbols(bot, msg).await,
                                Command::Balance { text: _ } => handle_balance(bot, msg, exchange_manager).await,
                                Command::Connect { text } => {
                                    let args = text.split_whitespace().map(String::from).collect();
                                    handle_connect(bot, msg, db, args).await
                                },
                                Command::Buy { text } => handle_buy(bot, msg, db.clone(), exchange_manager.clone(), text).await,
                                Command::Sell { text } => handle_sell(bot, msg, db.clone(), exchange_manager.clone(), text).await,
                                Command::Orders { text: _ } => handle_orders(bot, msg, exchange_manager).await,
                                Command::Cancel { text } => handle_cancel(bot, msg, text, exchange_manager).await,
                                Command::Order(text) => handle_order(bot, msg, text, exchange_manager).await,
                            }
                        }
                    }),
            );

        Self {
            bot,
            handler: handler.into(),
            db,
            exchange_manager,
        }
    }

    pub async fn run(&self) {
        Dispatcher::builder(self.bot.clone(), self.handler.clone())
            .dependencies(dptree::deps![self.db.clone(), self.exchange_manager.clone()])
            .default_handler(|upd| async move {
                log::warn!("Unhandled update: {:?}", upd);
            })
            .error_handler(LoggingErrorHandler::with_custom_text(
                "Error al procesar actualización"
            ))
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    }

    pub async fn get_user_by_chat_id(&self, chat_id: i64) -> Result<User, RequestError> {
        match self.db.get_user_by_telegram_id(chat_id).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => Err(RequestError::Api(ApiError::Unknown("Usuario no encontrado".into()))),
            Err(e) => Err(RequestError::Api(ApiError::Unknown(e.to_string())))
        }
    }
} 