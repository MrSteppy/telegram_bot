use std::default::Default;
use std::error::Error;
use std::fmt::{Display, Formatter};

use enum_assoc::Assoc;
use linkify::LinkFinder;

use crate::VarArgs;

pub fn parse(text: &str) -> Result<Vec<Component>, InvalidTagError> {
  let mut components: Vec<Component> = vec![];
  let mut open_tags: Vec<Decoration> = vec![];

  let mut token = String::new();
  let mut building_tag = false;
  let mut iter = text.chars().peekable();
  while let Some(c) = iter.next() {
    match c {
      '<' if !building_tag => {
        if !token.is_empty() {
          components.push(create_component(&token, &open_tags));
          token = String::new();
        }
        building_tag = true;
      }
      '>' if building_tag => {
        building_tag = false;
        let tag = create_tag(&token)?;
        token = String::new();
        if tag.closing {
          if let Some((index, _)) = open_tags
            .iter()
            .enumerate()
            .rfind(|(_, open_tag)| open_tag.name() == tag.decoration.name())
          {
            open_tags.remove(index);
          }
        } else {
          open_tags.push(tag.decoration);
        }
      }
      '\\'
        if iter
          .peek()
          .filter(|c| ['<', '>', '\\'].contains(c))
          .is_some() =>
      {
        token.push(iter.next().unwrap());
      }
      _ => token.push(c),
    }
  }
  if !building_tag {
    components.push(create_component(&token, &open_tags));
  } else {
    Err(InvalidTagError::new(format!(
      "missing closing bracket after '{}'",
      token
    )))?;
  }

  Ok(components)
}

fn create_component(content: &str, open_tags: &Vec<Decoration>) -> Component {
  Component::from(content)
    .style(Style::default().decorate(open_tags.iter().map(|tag| tag.clone()).collect::<Vec<_>>()))
}

fn create_tag(content: &str) -> Result<Tag, InvalidTagError> {
  let mut split = content.splitn(2, ':');
  let id = split.next().unwrap();
  let (name, closing) = if id.starts_with('/') {
    (&id[1..], true)
  } else {
    (id, false)
  };
  let mut decoration = Decoration::from(name).ok_or(InvalidTagError::new(content))?;
  if !closing {
    if let Decoration::Link(_) = &decoration {
      decoration = Decoration::Link(
        split
          .next()
          .ok_or(InvalidTagError::new("missing link target"))?
          .to_owned(),
      )
    }
  }
  Ok(Tag {
    decoration,
    closing,
  })
}

///escape all characters which might be interpreted as tags in a text
pub fn escape_tags(text: &str) -> String {
  text
    .replace('\\', "\\\\")
    .replace('<', "\\<")
    .replace('>', "\\>")
}

///surround all link urls in the text with a link tag pointing to the link
pub fn tag_links(text: &str) -> String {
  let mut tagged = String::new();
  for span in LinkFinder::new().spans(text) {
    if span.kind().is_some() {
      let link = span.as_str();
      let tag = Decoration::link(link);
      tagged += &format!("{}{}{}", tag.to_tag(false), link, tag.to_tag(true));
    } else {
      tagged += span.as_str();
    }
  }
  tagged
}

#[derive(Debug)]
pub struct InvalidTagError {
  tag: String,
}

impl InvalidTagError {
  pub fn new<S>(tag: S) -> Self
  where
    S: ToString,
  {
    InvalidTagError {
      tag: tag.to_string(),
    }
  }
}

impl Display for InvalidTagError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "invalid tag: '{}'", self.tag)
  }
}

impl Error for InvalidTagError {}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Component {
  pub text: String,
  pub style: Style,
}

impl Component {
  pub fn is_empty(&self) -> bool {
    self.text.is_empty()
  }

  pub fn style(mut self, style: Style) -> Self {
    self.style = style;
    self
  }

  pub fn decorate<C>(mut self, decorations: C) -> Self
  where
    C: VarArgs<Decoration>,
  {
    self.style = self.style.decorate(decorations);
    self
  }
}

impl<S> From<S> for Component
where
  S: ToString,
{
  fn from(value: S) -> Self {
    Self {
      text: value.to_string(),
      style: Style::default(),
    }
  }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
  tags: Vec<Decoration>,
}

impl Style {
  pub fn decorate<D>(mut self, decorations: D) -> Self
  where
    D: VarArgs<Decoration>,
  {
    for decoration in decorations.args() {
      if !self.tags.iter().any(|tag| tag.name() == decoration.name()) {
        self.tags.push(decoration);
      }
    }
    self
  }

  pub fn tags(&self) -> &Vec<Decoration> {
    &self.tags
  }
}

struct Tag {
  decoration: Decoration,
  closing: bool,
}

#[derive(Assoc, Clone, Debug, Eq, PartialEq, Hash)]
#[func(pub const fn name(& self) -> & 'static str)]
#[func(fn by_name(name: & str) -> Option < Self >)]
pub enum Decoration {
  #[assoc(name = "bold")]
  #[assoc(by_name = "bold")]
  Bold,
  #[assoc(name = "italic")]
  #[assoc(by_name = "italic")]
  Italic,
  #[assoc(name = "underline")]
  #[assoc(by_name = "underline")]
  #[assoc(by_name = "underlined")]
  Underlined,
  #[assoc(name = "mono-space")]
  #[assoc(by_name = "mono-space")]
  #[assoc(by_name = "code")]
  MonoSpace,
  #[assoc(name = "spoiler")]
  #[assoc(by_name = "spoiler")]
  Spoiler,
  #[assoc(name = "link")]
  Link(String),
}

impl Decoration {
  pub fn from(name: &str) -> Option<Self> {
    match name {
      "link" => Some(Self::Link(String::new())),
      _ => Self::by_name(name),
    }
  }

  pub fn link<S>(link: S) -> Self
  where
    S: ToString,
  {
    Self::Link(link.to_string())
  }

  pub fn to_tag(&self, closing: bool) -> String {
    if closing {
      format!("</{}>", self.name())
    } else {
      format!(
        "<{}{}>",
        self.name(),
        match self {
          Self::Link(link) => format!(":{}", link),
          _ => String::new(),
        }
      )
    }
  }
}

#[cfg(test)]
mod test {
  use crate::format::{escape_tags, parse, tag_links, Component, Decoration};

  #[test]
  fn test_decoration_from() {
    assert_eq!(Some(Decoration::Underlined), Decoration::from("underline"));
    assert_eq!(
      Some(Decoration::Link(String::new())),
      Decoration::from("link")
    );
  }

  #[test]
  fn test_parse() {
    let example_str =
      "<bold>Foo\\<T> <italic>bar</bold> buzz</italic> fee <spoiler>far <link:papermc.io>*klick*";
    let components = parse(example_str).expect("parse error");
    assert_eq!(
      vec![
        Component::from("Foo<T> ").decorate(Decoration::Bold),
        Component::from("bar").decorate([Decoration::Bold, Decoration::Italic]),
        Component::from(" buzz").decorate(Decoration::Italic),
        Component::from(" fee "),
        Component::from("far ").decorate(Decoration::Spoiler),
        Component::from("*klick*").decorate([Decoration::Spoiler, Decoration::link("papermc.io")]),
      ],
      components
    );

    let str = "foo <bold>bar</bold> buzz";
    assert_eq!(
      vec![
        Component::from("foo "),
        Component::from("bar").decorate([Decoration::Bold]),
        Component::from(" buzz"),
      ],
      parse(str).expect("parse error")
    );
  }

  #[test]
  fn test_escape_tags() {
    let text = "Foo<T> \\o/";
    let escaped = escape_tags(text);
    assert_eq!("Foo\\<T\\> \\\\o/", escaped);

    let components = parse(&escaped).expect("parse error");
    assert_eq!(1, components.len());
    assert_eq!(text, components[0].text);
  }

  #[test]
  fn test_decoration_to_tag() {
    assert_eq!("<bold>", Decoration::Bold.to_tag(false));
    assert_eq!("</underline>", Decoration::Underlined.to_tag(true));
    assert_eq!(
      "<link:papermc.io>",
      Decoration::link("papermc.io").to_tag(false)
    );
    assert_eq!("</link>", Decoration::link("papermc.io").to_tag(true));
  }

  #[test]
  fn test_tag_links() {
    let link = "https://papermc.io/";
    assert_eq!(format!("<link:{}>{}</link>", link, link), tag_links(link));
  }
}
