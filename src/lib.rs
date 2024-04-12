use std::sync::mpsc::{Receiver, SendError, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use teloxide::dispatching::dialogue::GetChatId;
use teloxide::prelude::*;
use teloxide::types as tg;
use teloxide::types::MessageId;
use tokio::runtime::Runtime;
use tokio::time;

use error::TelegramError;
use request::SendMessage;

use crate::error::ErrorKind;
use crate::update::{Query, UpdateKind, User};

pub mod error;
pub mod format;
pub mod request;
pub mod update;

pub type Result<T> = std::result::Result<T, TelegramError>;
pub type ChatID = i64;
pub type MessageID = i32;

#[derive(Debug, Clone)]
pub struct Bot {
  update_receiver: Arc<Receiver<Result<update::Update>>>,
  network_error_cooldown: Arc<Mutex<Duration>>,
  bot: teloxide::Bot,
  runtime: Arc<Runtime>,
}

impl Bot {
  pub fn new<S>(token: S) -> Result<Self>
  where
    S: Into<String>,
  {
    let bot = teloxide::Bot::new(token);
    let network_error_cooldown = Arc::new(Mutex::new(Duration::from_secs(2)));
    let nec_mutex = network_error_cooldown.clone();
    let (update_sender, update_receiver) = mpsc::channel();
    let update_receiver = Arc::new(update_receiver);
    let runtime = Arc::new(
      Runtime::new()
        .map_err(|e| TelegramError::new("failed to create tokio runtime").with_cause(e))?,
    );
    let poll_bot = bot.clone();
    runtime.spawn(async move {
      let mut ack: Option<i32> = None;
      while let Ok(_) = Self::poll(&poll_bot, &mut ack, &update_sender, || {
        nec_mutex.lock().unwrap().clone()
      })
      .await
      {}
    });
    let instance = Self {
      update_receiver,
      network_error_cooldown,
      bot,
      runtime,
    };
    Ok(instance)
  }

  async fn poll<F>(
    bot: &teloxide::Bot,
    ack: &mut Option<i32>,
    update_sender: &Sender<Result<update::Update>>,
    network_error_cooldown_supplier: F,
  ) -> std::result::Result<(), SendError<Result<update::Update>>>
  where
    F: Fn() -> Duration,
  {
    let mut get_updates = bot.get_updates();
    get_updates.offset = ack.map(|ack| ack + 1);
    get_updates.allowed_updates = Some(vec![]); //receive all updates
    match get_updates.await {
      Ok(updates) => {
        for update in updates {
          *ack = (*ack).max(Some(update.id));
          match update.kind {
            tg::UpdateKind::Message(message) => Self::wrap_message(message, false, &update_sender)?,
            tg::UpdateKind::EditedMessage(message) => {
              Self::wrap_message(message, true, &update_sender)?
            }
            tg::UpdateKind::CallbackQuery(callback_query) => {
              if let Some(query) = Query::from(&callback_query) {
                if let Some(chat_id) = callback_query.chat_id().map(|id| id.0) {
                  update_sender.send(Ok(update::Update {
                    user: User::from(&callback_query.from),
                    chat_id,
                    kind: UpdateKind::Query(query),
                  }))?;
                }
              }
            }
            _ => {}
          }
        }
      }
      Err(e) => {
        time::sleep(network_error_cooldown_supplier()).await;
        update_sender.send(Err(
          TelegramError::new("failed to poll updates")
            .of_kind(ErrorKind::Network)
            .with_cause(e),
        ))?;
      }
    }
    Ok(())
  }

  fn wrap_message(
    message: Message,
    edit: bool,
    update_sender: &Sender<Result<update::Update>>,
  ) -> std::result::Result<(), SendError<Result<update::Update>>> {
    if let Some(m) = update::Message::from(&message) {
      if let Some(user) = message.from() {
        update_sender.send(Ok(update::Update {
          user: User::from(user),
          chat_id: message.chat.id.0,
          kind: UpdateKind::Message { message: m, edit },
        }))?;
      }
    }
    Ok(())
  }

  pub fn send_message<I, S>(&self, chat_id: I, text: S) -> SendMessage
  where
    I: Into<ChatID>,
    S: ToString,
  {
    SendMessage::new(
      text.to_string(),
      chat_id.into(),
      self.bot.clone(),
      self.runtime.clone(),
    )
  }

  pub fn delete_message<I, M>(&self, chat_id: I, message_id: M) -> Result<()>
  where
    I: Into<ChatID>,
    M: Into<MessageID>,
  {
    self.runtime.block_on(async move {
      self
        .bot
        .delete_message(ChatId(chat_id.into()), MessageId(message_id.into()))
        .await
        .map_err(|e| TelegramError::new("failed to delete message").with_cause(e))
    })?;
    Ok(())
  }

  pub fn poll_update(&self) -> Option<Result<update::Update>> {
    self.update_receiver.try_recv().ok()
  }

  pub fn await_update(&self) -> Result<update::Update> {
    self
      .update_receiver
      .recv()
      .map_err(|e| TelegramError::new("update sender has gone out of scope").with_cause(e))
      .and_then(|r| r)
  }

  pub fn await_update_with_timeout(&self, time_out: Duration) -> Option<Result<update::Update>> {
    self.update_receiver.recv_timeout(time_out).ok()
  }

  pub fn get_network_error_cooldown(&self) -> Duration {
    self.network_error_cooldown.lock().unwrap().clone()
  }

  pub fn set_network_error_cooldown(&mut self, network_error_cooldown: Duration) {
    *self.network_error_cooldown.lock().unwrap() = network_error_cooldown;
  }
}

pub trait VarArgs<T> {
  fn args(self) -> Vec<T>;
}

impl<T> VarArgs<T> for T {
  fn args(self) -> Vec<T> {
    vec![self]
  }
}

impl<T, const N: usize> VarArgs<T> for [T; N] {
  fn args(self) -> Vec<T> {
    self.into_iter().collect()
  }
}

impl<T> VarArgs<T> for Vec<T> {
  fn args(self) -> Vec<T> {
    self
  }
}
