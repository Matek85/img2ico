// What the conversion has to tell about the file it is working on, without
// printing it or knowing who listens: warnings (counted, and handed to
// whoever asked to see them) and the "which file is this about" prefix for
// messages. The command line prints the warnings to standard error; the web
// page collects them (see `collect`).

use std::cell::{Cell, RefCell};
use std::path::Path;

thread_local! {
    /// The input file this thread is converting right now - set only while
    /// a batch run works through several files (see `enter_file_context`).
    /// Per thread, so it stays correct if files are ever processed in
    /// parallel.
    static CURRENT_FILE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Marks `path` as the file being converted until the returned guard is
/// dropped. While it is alive, `file_prefix()` yields `"<path>: "`, so a
/// warning raised deep inside the conversion (an upscaling hint, a seed
/// outside the image, ...) says which of the many files it is about.
#[must_use = "the file context ends as soon as this guard is dropped"]
pub fn enter_file_context(path: &Path) -> FileContext {
    CURRENT_FILE.with(|current| *current.borrow_mut() = Some(path.display().to_string()));
    FileContext(())
}

/// Ends the context begun by `enter_file_context` when dropped.
pub struct FileContext(());

impl Drop for FileContext {
    fn drop(&mut self) {
        CURRENT_FILE.with(|current| *current.borrow_mut() = None);
    }
}

thread_local! {
    /// How many warnings this thread has raised so far (see `warn`).
    static WARNINGS: Cell<usize> = const { Cell::new(0) };
}

thread_local! {
    /// The warnings gathered by `collect`, while it is running on this thread.
    static COLLECTED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Raises a warning: counts it - always, so a batch summary can say how many
/// there were even when --silent hid them - and then either hands `message`
/// to a `collect` that is running on this thread, or prints it to standard
/// error unless `silent` is set. `message` is used as given, so it carries
/// its own "Warning: " lead-in where it wants one.
pub fn warn(silent: bool, message: impl std::fmt::Display) {
    WARNINGS.with(|count| count.set(count.get() + 1));
    let collected = COLLECTED.with(|slot| match slot.borrow_mut().as_mut() {
        Some(list) => {
            list.push(message.to_string());
            true
        }
        None => false,
    });
    if !collected && !silent {
        eprintln!("{message}");
    }
}

/// Runs `work` and returns its result together with the warnings it raised
/// on this thread, instead of printing them. This is how a caller that has no
/// standard error - the web page - gets to see them.
pub fn collect<R>(work: impl FnOnce() -> R) -> (R, Vec<String>) {
    struct Restore(Option<Vec<String>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            COLLECTED.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let outer = COLLECTED.with(|slot| slot.borrow_mut().replace(Vec::new()));
    let restore = Restore(outer);
    let result = work();
    let warnings = COLLECTED
        .with(|slot| slot.borrow_mut().take())
        .unwrap_or_default();
    drop(restore);
    (result, warnings)
}

/// The number of warnings raised on this thread so far. A caller that wants
/// the warnings of one piece of work reads it before and after.
pub fn warnings_so_far() -> usize {
    WARNINGS.with(Cell::get)
}

/// The text to put in front of a per-file message: `"logo.png: "` while a
/// file context is active (batch mode), and an empty string otherwise - so
/// messages from a single conversion read exactly as they always have.
pub fn file_prefix() -> String {
    CURRENT_FILE.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|path| format!("{path}: "))
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- file context ----------------------------------------------------------

    #[test]
    fn there_is_no_prefix_outside_a_file_context() {
        assert_eq!(file_prefix(), "");
    }

    #[test]
    fn a_file_context_adds_the_path_as_a_prefix_until_it_ends() {
        {
            let _context = enter_file_context(Path::new("art/logo.png"));
            assert_eq!(
                file_prefix(),
                format!("{}: ", Path::new("art/logo.png").display())
            );
        }
        assert_eq!(
            file_prefix(),
            "",
            "the prefix must disappear with the guard"
        );
    }

    #[test]
    fn a_new_file_context_replaces_the_previous_one() {
        let first = enter_file_context(Path::new("a.png"));
        drop(first);
        let _second = enter_file_context(Path::new("b.png"));
        assert!(file_prefix().starts_with("b.png"), "{}", file_prefix());
    }

    #[test]
    fn the_file_context_belongs_to_one_thread() {
        let _context = enter_file_context(Path::new("main-thread.png"));
        let other = std::thread::spawn(file_prefix).join().unwrap();
        assert_eq!(other, "", "another thread must not see this thread's file");
        assert!(file_prefix().starts_with("main-thread.png"));
    }

    #[test]
    fn collect_hands_over_the_warnings_instead_of_printing_them() {
        let before = warnings_so_far();
        let (value, warnings) = collect(|| {
            warn(false, "Warning: one");
            warn(true, "Warning: two (even a silenced one is collected)");
            7
        });
        assert_eq!(value, 7);
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0], "Warning: one");
        assert_eq!(warnings_so_far() - before, 2, "they are counted as well");
    }

    #[test]
    fn collect_calls_nest_and_do_not_leak() {
        let (_, outer) = collect(|| {
            warn(false, "outer 1");
            let (_, inner) = collect(|| warn(false, "inner"));
            assert_eq!(inner, vec!["inner".to_string()]);
            warn(false, "outer 2");
        });
        assert_eq!(outer, vec!["outer 1".to_string(), "outer 2".to_string()]);
        let (_, after) = collect(|| ());
        assert!(after.is_empty());
    }
}
