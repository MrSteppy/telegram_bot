use teloxide::dispatching::dialogue::GetChatId;
use teloxide::prelude::*;

use crate::{ChatID, MessageID};

#[derive(Debug)]
pub struct Update {
  pub chat_id: ChatID,
  pub user: User,
  pub kind: UpdateKind,
}

#[derive(Debug)]
pub enum UpdateKind {
  Message { message: Message, edit: bool },
  Query(Query),
}

#[derive(Debug, Clone)]
pub struct Message {
  pub id: MessageID,
  pub text: String,
  pub replying_to: Option<Box<Message>>,
}

impl Message {
  pub fn from(message: &teloxide::types::Message) -> Option<Self> {
    Self {
      id: message.id.0,
      text: message.text()?.to_owned(),
      replying_to: message
        .reply_to_message()
        .and_then(|message| Self::from(message).map(|message| Box::new(message))),
    }
    .into()
  }
}

#[derive(Debug)]
pub struct Query {
  pub text: String,
  pub message: Message,
  pub from: User,
  pub chat_id: ChatID,
}

impl Query {
  pub fn from(callback_query: &CallbackQuery) -> Option<Self> {
    Self {
      text: callback_query.data.as_ref()?.to_owned(),
      message: Message::from(callback_query.message.as_ref()?)?,
      from: User::from(&callback_query.from),
      chat_id: callback_query.chat_id()?.0,
    }
    .into()
  }
}

#[derive(Debug)]
pub struct User {
  pub id: ChatID,
  pub user_name: Option<String>,
  pub first_name: String,
  pub last_name: Option<String>,
}

impl User {
  pub fn from(user: &teloxide::types::User) -> Self {
    Self {
      id: user.id.0 as ChatID,
      user_name: user.username.to_owned(),
      first_name: user.first_name.to_owned(),
      last_name: user.last_name.to_owned(),
    }
  }
}

impl User {
  pub fn full_name(&self) -> String {
    format!(
      "{}{}",
      &self.first_name,
      self.last_name.as_ref().unwrap_or(&String::new())
    )
  }
}
