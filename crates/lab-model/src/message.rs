//! User-facing messages: a stable code, named values and the canonical English text.
//!
//! The backend decides what a message says; the interface decides how it is worded in
//! the viewer's language. A message therefore carries its code and the values that fill
//! the wording, and keeps the English text that the backend has always produced, so a
//! viewer without a translation for a code still reads the exact original.
//!
//! Build messages with [`message!`](crate::message!): the English text is formatted from
//! the same named values that travel with the code.

use std::fmt;

/// A value inside a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// Shown as it is: names, tags, addresses, numbers.
    Data(String),
    /// A word from a closed vocabulary (a quality, an LED state, a signal type) that the
    /// interface translates by its code; `english` is the canonical word.
    Term { code: String, english: String },
    /// A nested message, for example a list item worded on its own.
    Message(Box<Message>),
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(text) => f.write_str(text),
            Self::Term { english, .. } => f.write_str(english),
            Self::Message(message) => f.write_str(&message.text),
        }
    }
}

/// Conversion of message values: text and numbers become data; terms and messages stay.
pub trait IntoArg {
    fn into_arg(self) -> Arg;
}

impl IntoArg for Arg {
    fn into_arg(self) -> Arg {
        self
    }
}

impl IntoArg for Message {
    fn into_arg(self) -> Arg {
        Arg::Message(Box::new(self))
    }
}

macro_rules! data_arg {
    ($($t:ty),*) => {$(
        impl IntoArg for $t {
            fn into_arg(self) -> Arg {
                Arg::Data(self.to_string())
            }
        }
    )*};
}
data_arg!(String, &str, &String, u8, u16, u32, u64, usize, i32, i64);

/// A vocabulary term: `code` is stable, `english` is the canonical word.
pub fn term(code: impl Into<String>, english: impl Into<String>) -> Arg {
    Arg::Term {
        code: code.into(),
        english: english.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Stable code, for example `"power.drive.supplied"`.
    pub code: &'static str,
    /// Named values in the order they were given.
    pub args: Vec<(&'static str, Arg)>,
    /// The canonical English text.
    pub text: String,
}

impl Message {
    /// A message without values.
    pub fn fixed(code: &'static str, text: &'static str) -> Self {
        Self {
            code,
            args: Vec::new(),
            text: text.to_owned(),
        }
    }
}

impl Message {
    /// Items joined by ", ": code `"list"`, one `"item"` value per entry, so the interface
    /// can word each entry and join them itself.
    pub fn list(items: Vec<Arg>) -> Self {
        let text = items
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            code: "list",
            args: items.into_iter().map(|a| ("item", a)).collect(),
            text,
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// `message!("code", "English with {name}", name = value, ...)`.
///
/// Placeholders are plain `{name}`: format numbers before passing them, so the English
/// text and every translation show the same digits.
#[macro_export]
macro_rules! message {
    ($code:literal, $text:literal $(,)?) => {
        $crate::message::Message::fixed($code, $text)
    };
    ($code:literal, $text:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        $(let $name = $crate::message::IntoArg::into_arg($value);)+
        $crate::message::Message {
            code: $code,
            text: format!($text, $($name = $name),+),
            args: vec![$((stringify!($name), $name)),+],
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_english_text_is_formatted_from_the_values_that_travel_with_the_code() {
        let quality = term("quality.WIRE_BREAK", "wire break");
        let m = crate::message!(
            "input.quality",
            "{label}: quality {quality} at {count} channel(s)",
            label = "LT-203",
            quality = quality,
            count = 2u8,
        );
        assert_eq!(m.code, "input.quality");
        assert_eq!(m.text, "LT-203: quality wire break at 2 channel(s)");
        assert_eq!(
            m.args,
            vec![
                ("label", Arg::Data("LT-203".into())),
                ("quality", term("quality.WIRE_BREAK", "wire break")),
                ("count", Arg::Data("2".into())),
            ]
        );
        assert_eq!(crate::message!("x.fixed", "No values.").text, "No values.");
    }

    #[test]
    fn a_nested_message_is_shown_by_its_text_and_kept_as_a_message() {
        let inner = crate::message!("a.inner", "inner {n}", n = 1u8);
        let outer = crate::message!("a.outer", "outer: {inner}", inner = inner.clone());
        assert_eq!(outer.text, "outer: inner 1");
        assert_eq!(outer.args, vec![("inner", Arg::Message(Box::new(inner)))]);
        let list = Message::list(vec![term("a.x", "x"), Arg::Data("y".into())]);
        assert_eq!((list.code, list.text.as_str()), ("list", "x, y"));
    }
}
