// Messages that can be told apart by a code. The command line shows the English
// sentence; the web page wants the code and the values in it, so that it can write
// the sentence in the visitor's language. `msg!` makes the sentence (it is exactly
// what `format!` would make) and, where something asked for it, remembers which
// code and values it was made from, so the page can look that up from the text it
// is handed. Nothing is remembered in the command line: it never asks.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// A message as code and values: `code` is a stable name such as `chroma.nothing_removed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structured {
    pub code: &'static str,
    pub params: Vec<(&'static str, String)>,
}

/// How many messages are remembered at most (a long visit may raise many different ones).
const REMEMBERED: usize = 4000;

thread_local! {
    static REMEMBERING: Cell<bool> = const { Cell::new(false) };
    static KNOWN: RefCell<HashMap<String, Structured>> = RefCell::new(HashMap::new());
}

/// Starts remembering the code and values of every message made on this thread.
pub fn remember_messages() {
    REMEMBERING.with(|flag| flag.set(true));
}

/// Called by `msg!`; does nothing unless `remember_messages` was called on this thread.
pub fn record(code: &'static str, params: &[(&'static str, String)], text: &str) {
    if !REMEMBERING.with(Cell::get) {
        return;
    }
    KNOWN.with(|known| {
        let mut known = known.borrow_mut();
        if known.len() >= REMEMBERED {
            known.clear();
        }
        known.insert(
            text.to_string(),
            Structured {
                code,
                params: params.to_vec(),
            },
        );
    });
}

/// The code and values `text` was made from, if it came from `msg!` and was remembered.
pub fn lookup(text: &str) -> Option<Structured> {
    KNOWN.with(|known| known.borrow().get(text).cloned())
}

/// Makes a message from a code and a `format!` template, naming the values it uses:
/// `msg!("example.too_big", "{width}x{height} is too big", width = w, height = h)`.
/// The result is the plain `String` the template makes. Every value named must be used
/// by the template, and is given to the page as text.
#[macro_export]
macro_rules! msg {
    ($code:literal, $template:literal $(, $name:ident = $value:expr)* $(,)?) => {{
        $( let $name = &$value; )*
        let text = format!($template $(, $name = $name)*);
        $crate::msg::record($code, &[$((stringify!($name), $name.to_string())),*], &text);
        text
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_message_is_its_sentence_and_is_remembered_only_when_asked() {
        let before = msg!(
            "test.size",
            "{width}x{height} pixels",
            width = 3,
            height = 4
        );
        assert_eq!(before, "3x4 pixels");
        assert!(super::lookup("3x4 pixels").is_none());

        super::remember_messages();
        let text = msg!(
            "test.size",
            "{width}x{height} pixels",
            width = 5,
            height = 6
        );
        assert_eq!(text, "5x6 pixels");
        let known = super::lookup("5x6 pixels").unwrap();
        assert_eq!(known.code, "test.size");
        assert_eq!(
            known.params,
            vec![("width", "5".to_string()), ("height", "6".to_string())]
        );
        assert!(super::lookup("something else").is_none());
    }
}
