use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct TelegramError {
  pub kind: ErrorKind,
  pub detail_message: String,
  pub cause: Option<Box<dyn Error + Sync + Send>>,
}

impl TelegramError {
  pub fn new<S>(detail_message: S) -> Self
  where
    S: ToString,
  {
    Self {
      kind: ErrorKind::default(),
      detail_message: detail_message.to_string(),
      cause: None,
    }
  }

  pub fn with_cause<E>(mut self, cause: E) -> Self
  where
    E: Into<Box<dyn Error + Sync + Send>>,
  {
    self.cause = Some(cause.into());
    self
  }

  pub fn of_kind<K>(mut self, kind: K) -> Self
  where
    K: Into<ErrorKind>,
  {
    self.kind = kind.into();
    self
  }
}

impl Display for TelegramError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "telegram error{}: {}",
      Some(self.kind)
        .filter(|kind| kind != &ErrorKind::Other)
        .map(|kind| format!(" ({})", kind))
        .unwrap_or_default(),
      self.detail_message
    )?;
    if let Some(cause) = &self.cause {
      write!(f, "\ncaused by {}", cause)?;
    }
    Ok(())
  }
}

impl Error for TelegramError {}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ErrorKind {
  /**
   * The telegram api is unreachable
   */
  Network,
  MessageCharLimitReached,
  QueryByteLimitReached,
  #[default]
  Other,
}

impl Display for ErrorKind {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}
