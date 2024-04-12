use std::fs;

use telegram_bot::request::Button;
use telegram_bot::update::UpdateKind;
use telegram_bot::Bot;

fn main() {
  let token = fs::read_to_string("token.txt").expect("token file missing");
  let bot = Bot::new(token).expect("failed to create bot");
  let mut counter = 0;
  loop {
    match bot.await_update() {
      Ok(update) => match &update.kind {
        UpdateKind::Message { message, .. } => {
          if let Err(e) = bot
            .send_message(update.chat_id, format!("Current count: <code>{}", counter))
            .reply_to(message)
            .add_button(Button::new("Increase count", "increase"))
            .execute()
          {
            eprintln!("{}", e);
          }
        }
        UpdateKind::Query(query) => {
          match query.text.as_str() {
            "increase" => {
              counter += 1;
              if let Err(e) = bot
                .send_message(
                  update.chat_id,
                  format!(
                    "counter <bold>increased</bold>\n\ncurrent count: <code>{}",
                    counter
                  ),
                )
                .add_button(Button::new("Increase count", "increase"))
                .execute()
              {
                eprintln!("{}", e);
              }
            }
            _ => {
              eprintln!("received invalid query: {}", query.text);
            }
          };
        }
      },
      Err(e) => {
        eprintln!("{}", e);
      }
    }
  }
}
