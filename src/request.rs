use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::{
  InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode, ReplyMarkup,
};
use tokio::runtime::Runtime;

use crate::error::{ErrorKind, TelegramError};
use crate::format::{parse, Decoration};
use crate::update::Message;
use crate::{ChatID, VarArgs};

pub const MESSAGE_CHAR_LIMIT: u32 = 4096;
pub const QUERY_BYTE_LIMIT: u32 = 64;

#[derive(Debug)]
pub struct SendMessage {
  text: String,
  send_to: ChatID,
  bot: Bot,
  runtime: Arc<Runtime>,
  reply_to: Option<Message>,
  buttons: Vec<Vec<Button>>,
}

impl SendMessage {
  pub(crate) fn new(text: String, send_to: ChatID, bot: Bot, runtime: Arc<Runtime>) -> Self {
    Self {
      text,
      send_to,
      bot,
      runtime,
      reply_to: None,
      buttons: vec![],
    }
  }

  pub fn reply_to(mut self, message: &Message) -> Self {
    self.reply_to = Some(message.clone());
    self
  }

  pub fn buttons<B>(mut self, buttons: Vec<B>) -> Self
  where
    B: VarArgs<Button>,
  {
    let buttons: Vec<Vec<Button>> = buttons.into_iter().map(|line| line.args()).collect();
    self.buttons = buttons;
    self
  }

  pub fn add_button<B>(mut self, buttons: B) -> Self
  where
    B: VarArgs<Button>,
  {
    if let Some(row) = self.buttons.last_mut() {
      row.append(&mut buttons.args());
    } else {
      self = self.add_button_row(buttons);
    }
    self
  }

  pub fn add_button_row<B>(mut self, buttons: B) -> Self
  where
    B: VarArgs<Button>,
  {
    self.buttons.push(buttons.args());
    self
  }

  pub fn execute(&self) -> crate::Result<()> {
    //convert message text format
    let text = to_html(&self.text)?;

    let char_count = text.chars().count();
    if char_count > MESSAGE_CHAR_LIMIT as usize {
      Err(
        TelegramError::new(format!(
          "message char count ({}) exceeds limit ({})",
          char_count, MESSAGE_CHAR_LIMIT
        ))
        .of_kind(ErrorKind::MessageCharLimitReached),
      )?;
    }
    let mut send_message = self
      .bot
      .send_message(ChatId(self.send_to), &text)
      .parse_mode(ParseMode::Html);

    if let Some(reply_to) = &self.reply_to {
      send_message.reply_to_message_id = Some(MessageId(reply_to.id));
    }

    for button in self.buttons.iter().flat_map(|row| row) {
      let bytes = button.query.len();
      if bytes > QUERY_BYTE_LIMIT as usize {
        Err(TelegramError::new(format!(
          "query size ({} bytes) for button {:?} exceeds limit ({} bytes)",
          bytes, button, QUERY_BYTE_LIMIT
        )))?;
      }
    }
    send_message.reply_markup = Some(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup {
      inline_keyboard: self
        .buttons
        .iter()
        .map(|row| {
          row
            .iter()
            .map(|button| InlineKeyboardButton::callback(&button.text, &button.query))
            .collect()
        })
        .collect(),
    }));

    self
      .runtime
      .block_on(async move { send_message.await })
      .map_err(|e| TelegramError::new("failed to send message").with_cause(e))?;

    Ok(())
  }
}

fn to_html(text: &str) -> Result<String, TelegramError> {
  Ok(
    parse(text)
      .map_err(|e| TelegramError::new("invalid format tag").with_cause(e))?
      .into_iter()
      .map(|component| {
        let mut opened_html_tags = vec![];
        let mut part = String::new();
        for tag in component.style.tags() {
          part += &format!(
            "<{}>",
            match tag {
              Decoration::Bold => {
                opened_html_tags.push("b");
                "b".to_owned()
              }
              Decoration::Italic => {
                opened_html_tags.push("i");
                "i".to_owned()
              }
              Decoration::Underlined => {
                opened_html_tags.push("u");
                "u".to_owned()
              }
              Decoration::MonoSpace => {
                opened_html_tags.push("code");
                "code".to_owned()
              }
              Decoration::Spoiler => {
                opened_html_tags.push("tg-spoiler");
                "tg-spoiler".to_owned()
              }
              Decoration::Link(link) => {
                opened_html_tags.push("a");
                format!("a href=\"{}\"", link)
              }
            }
          );
        }
        part += &component
          .text
          .replace('&', "&amp;")
          .replace('<', "&lt;")
          .replace('>', "&gt;");
        for tag in opened_html_tags {
          part += &format!("</{}>", tag);
        }
        part
      })
      .collect::<Vec<_>>()
      .join(""),
  )
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Button {
  pub text: String,
  pub query: String,
}

impl Button {
  pub fn new<T, Q>(text: T, query: Q) -> Self
  where
    T: ToString,
    Q: ToString,
  {
    Self {
      text: text.to_string(),
      query: query.to_string(),
    }
  }
}

#[cfg(test)]
mod test {
  use crate::request::to_html;

  #[test]
  fn test_to_html() {
    assert_eq!(
      "foo <b>bar</b> buzz",
      to_html("foo <bold>bar</bold> buzz").expect("format error")
    );
  }
}
