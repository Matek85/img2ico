// Small, general-purpose helpers that don't belong to any one feature:
// parsing a "--seed" value, the shared overwrite-protection check, safe
// source-file deletion for --delete-source, and the "which file is this
// message about" context used while converting several files in a row.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

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

/// Whether --quiet is on: the normal progress and success output is off.
static QUIET: AtomicBool = AtomicBool::new(false);

/// Whether standard output is reserved for the icon itself (`-o -`), so that
/// no text may be written to it.
static STDOUT_RESERVED: AtomicBool = AtomicBool::new(false);

/// Where a line of the run's own commentary goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sink {
    Stdout,
    Stderr,
    Nowhere,
}

/// Where commentary goes for the given settings: nowhere with --quiet,
/// standard error when standard output carries the icon, else standard
/// output.
pub fn sink_for(quiet: bool, stdout_reserved: bool) -> Sink {
    if quiet {
        Sink::Nowhere
    } else if stdout_reserved {
        Sink::Stderr
    } else {
        Sink::Stdout
    }
}

/// Sets how `say` behaves for the rest of the run. Called once, right after
/// the command line is parsed.
pub fn set_output_mode(quiet: bool, stdout_reserved: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
    STDOUT_RESERVED.store(stdout_reserved, Ordering::Relaxed);
}

/// Prints one line of the run's own commentary - "Done: ...", progress, the
/// batch summary, notices about settings files. This is what --quiet turns
/// off, and what moves to standard error when standard output is reserved for
/// the icon (`-o -`). The requested output of a mode - an --inspect or
/// --find report, a --what-if rehearsal - is printed with `println!` instead:
/// it is the point of the run, not commentary on it.
pub fn say(line: std::fmt::Arguments) {
    match sink_for(
        QUIET.load(Ordering::Relaxed),
        STDOUT_RESERVED.load(Ordering::Relaxed),
    ) {
        Sink::Stdout => println!("{line}"),
        Sink::Stderr => eprintln!("{line}"),
        Sink::Nowhere => {}
    }
}

thread_local! {
    /// How many warnings this thread has raised so far (see `warn`).
    static WARNINGS: Cell<usize> = const { Cell::new(0) };
}

/// Raises a warning: counts it - always, so a batch summary can say how many
/// there were even when --silent hid them - and prints `message` to standard
/// error unless `silent` is set. `message` is printed as given, so it
/// carries its own "Warning: " lead-in where it wants one.
pub fn warn(silent: bool, message: impl std::fmt::Display) {
    WARNINGS.with(|count| count.set(count.get() + 1));
    if !silent {
        eprintln!("{message}");
    }
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

/// Parses a "--seed" value in the format "x,y" (e.g. "200,50") into a
/// coordinate pair. Returns an understandable error message if the format
/// doesn't match.
pub fn parse_seed(input: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = input.split(',').map(str::trim).collect();
    if parts.len() != 2 {
        return Err(format!(
            "'{input}' is not a valid seed point (expected format is 'x,y', e.g. '200,50')"
        ));
    }
    let x = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("'{}' is not a valid x coordinate", parts[0]))?;
    let y = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("'{}' is not a valid y coordinate", parts[1]))?;
    Ok((x, y))
}

/// Refuses to silently overwrite an existing file: returns an error
/// (naming --force as the way around it) if `path` already exists and
/// `force` is false. A no-op if `force` is true, or if nothing exists at
/// `path` yet, so callers can just unconditionally call this right before
/// writing.
pub fn check_overwrite(path: &Path, force: bool) -> Result<(), String> {
    if !force && path.exists() {
        return Err(format!(
            "'{}' already exists. Use --force to overwrite it.",
            path.display()
        ));
    }
    Ok(())
}

/// How many threads to use when the user did not say: one per processor
/// (1 if that cannot be told).
pub fn available_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// Runs `work` for every item - on up to `threads` threads - and returns the
/// results in the order of `items`, whatever order they finished in. With one
/// thread (or one item) nothing is spawned and it is a plain loop.
pub fn parallel_map<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    work: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    use std::sync::Mutex;
    use std::sync::atomic::AtomicUsize;

    if threads <= 1 || items.len() <= 1 {
        return items.iter().map(work).collect();
    }
    let next = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<R>>> = items.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..threads.min(items.len()) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(index) else { break };
                    let result = work(item);
                    *slots[index]
                        .lock()
                        .expect("a result slot is never poisoned") = Some(result);
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|slot| {
            slot.into_inner()
                .expect("a result slot is never poisoned")
                .expect("every item has been worked on")
        })
        .collect()
}

/// Runs `work` for the items on up to `threads` threads and hands the results
/// to `handle` strictly in the order of `items` - on the calling thread, as
/// soon as the next one in line is ready - so progress can be reported in a
/// stable order while the work itself finishes in any order.
///
/// `handle` returns whether to go on. When it returns false no NEW item is
/// started any more, but the ones already running are finished and handed
/// over too (their work has been done; it must be accounted for). Items are
/// started in order, so what was started is always a prefix of `items`. With
/// one thread (or one item) it is a plain loop, and `false` ends it at once.
pub fn for_each_ordered<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    work: impl Fn(usize, &T) -> R + Sync,
    mut handle: impl FnMut(usize, R) -> bool,
) {
    use std::collections::BTreeMap;
    use std::sync::atomic::AtomicUsize;

    if threads <= 1 || items.len() <= 1 {
        for (index, item) in items.iter().enumerate() {
            if !handle(index, work(index, item)) {
                break;
            }
        }
        return;
    }

    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..threads.min(items.len()) {
            let sender = sender.clone();
            let (next, stop, work) = (&next, &stop, &work);
            scope.spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(index) else { break };
                    // The receiver outlives the workers; a failed send only
                    // means the run is over.
                    let _ = sender.send((index, work(index, item)));
                }
            });
        }
        drop(sender);

        let mut waiting: BTreeMap<usize, R> = BTreeMap::new();
        let mut expected = 0;
        for (index, result) in receiver {
            waiting.insert(index, result);
            while let Some(result) = waiting.remove(&expected) {
                if !handle(expected, result) {
                    stop.store(true, Ordering::Relaxed);
                }
                expected += 1;
            }
        }
    });
}

/// A limit on how much work of one kind - here: pixels of decoded images -
/// is under way at the same time across threads, so that many workers
/// converting large photos together cannot use up the memory of the machine.
pub struct Budget {
    capacity: u64,
    in_use: std::sync::Mutex<u64>,
    freed: std::sync::Condvar,
}

/// What a thread holds of a `Budget` until it is dropped.
#[must_use = "the share is given back as soon as this is dropped"]
pub struct Share<'a> {
    budget: &'a Budget,
    amount: u64,
}

impl Budget {
    pub fn new(capacity: u64) -> Self {
        Self {
            capacity,
            in_use: std::sync::Mutex::new(0),
            freed: std::sync::Condvar::new(),
        }
    }

    /// Waits until `amount` fits next to what others hold, then holds it.
    /// An amount larger than the whole budget is not turned away - it waits
    /// until nobody else holds anything and then runs alone - so every job
    /// gets its turn.
    pub fn take(&self, amount: u64) -> Share<'_> {
        let mut in_use = self.in_use.lock().expect("the budget is never poisoned");
        while *in_use != 0 && *in_use + amount > self.capacity {
            in_use = self
                .freed
                .wait(in_use)
                .expect("the budget is never poisoned");
        }
        *in_use += amount;
        Share {
            budget: self,
            amount,
        }
    }
}

impl Drop for Share<'_> {
    fn drop(&mut self) {
        let mut in_use = self
            .budget
            .in_use
            .lock()
            .expect("the budget is never poisoned");
        *in_use -= self.amount;
        self.budget.freed.notify_all();
    }
}

/// Writes `bytes` to `path` so that the file is either the complete new
/// content or - if anything goes wrong, or the program is killed half way -
/// what it was before (or not there at all), never a half-written one. The
/// bytes go to a temporary file in the same folder first and are renamed over
/// `path` only when all of them are on disk; the rename is a single step for
/// the file system. The temporary file is removed again if the write fails.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let folder = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("the path has no file name"))?
        .to_string_lossy();
    let temporary = folder.join(format!(
        ".{name}.{}-{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let written = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        // The data has to be on disk before the rename makes it the file.
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

/// Checks whether two paths point at the same file on disk, resolving
/// symlinks/relative components first (`/a/./b.png` and `/a/b.png` should
/// count as "the same file" even though they're different strings).
/// Falls back to plain path equality if either path can't be resolved
/// (e.g. because it doesn't exist) - a non-existent path can't be "the
/// same file" as anything by resolution anyway, so plain comparison is a
/// reasonable fallback.
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Deletes the given source files as part of --delete-source, skipping
/// (with a warning) any file that is also the resolved output path -
/// that would otherwise delete the just-written result instead of a
/// source file. `output_path` is `None` for extract mode, where the
/// output is a directory and can never collide with a source file path.
///
/// A failed deletion (e.g. permission denied) only prints a warning and
/// does not turn the overall command into a failure: the actual
/// conversion/merge/extraction the user asked for already succeeded by
/// the time this runs, so a cleanup problem afterward shouldn't make the
/// whole command look like it failed.
pub fn delete_source_files(paths: &[PathBuf], output_path: Option<&Path>, silent: bool) {
    for path in paths {
        if let Some(output_path) = output_path
            && same_file(path, output_path)
        {
            if !silent {
                eprintln!(
                    "Not deleting '{}': it's also the output path.",
                    path.display()
                );
            }
            continue;
        }
        match std::fs::remove_file(path) {
            Ok(()) => say!("Deleted source file '{}'.", path.display()),
            Err(e) => {
                if !silent {
                    eprintln!("Warning: could not delete '{}': {e}", path.display());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_budget_never_lets_more_than_its_capacity_run_together() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let budget = super::Budget::new(100);
        let running = AtomicU64::new(0);
        let highest = AtomicU64::new(0);
        super::parallel_map(&[40u64; 12], 8, |&amount| {
            let _share = budget.take(amount);
            let now = running.fetch_add(amount, Ordering::SeqCst) + amount;
            highest.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(5));
            running.fetch_sub(amount, Ordering::SeqCst);
        });
        assert!(highest.load(Ordering::SeqCst) <= 100, "{highest:?}");
        assert!(highest.load(Ordering::SeqCst) >= 40);
    }

    #[test]
    fn an_amount_beyond_the_budget_still_gets_its_turn_alone() {
        let budget = super::Budget::new(10);
        let results = super::parallel_map(&[5u64, 500, 5, 500], 4, |&amount| {
            let _share = budget.take(amount);
            amount
        });
        assert_eq!(results, vec![5, 500, 5, 500]);
    }

    #[test]
    fn parallel_map_keeps_the_order_of_the_items() {
        let items: Vec<u32> = (0..50).collect();
        for threads in [1, 2, 8, 100] {
            let squares = super::parallel_map(&items, threads, |&n| {
                // Make the early items the slow ones, so they finish last.
                std::thread::sleep(std::time::Duration::from_micros(u64::from(50 - n) * 20));
                n * n
            });
            assert_eq!(
                squares,
                items.iter().map(|n| n * n).collect::<Vec<_>>(),
                "{threads}"
            );
        }
        assert!(super::parallel_map(&Vec::<u32>::new(), 4, |&n| n).is_empty());
    }

    #[test]
    fn parallel_map_really_uses_several_threads() {
        let ids = super::parallel_map(&[0; 8], 4, |_| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            std::thread::current().id()
        });
        let distinct: std::collections::HashSet<_> = ids.into_iter().collect();
        assert!(distinct.len() > 1, "all on one thread: {distinct:?}");
    }

    #[test]
    fn for_each_ordered_hands_over_in_order_whatever_the_finishing_order() {
        let items: Vec<u32> = (0..40).collect();
        for threads in [1, 3, 16] {
            let mut seen = Vec::new();
            super::for_each_ordered(
                &items,
                threads,
                |_, &n| {
                    std::thread::sleep(std::time::Duration::from_micros(u64::from(40 - n) * 30));
                    n * 2
                },
                |index, result| {
                    seen.push((index, result));
                    true
                },
            );
            let expected: Vec<(usize, u32)> = items.iter().map(|&n| (n as usize, n * 2)).collect();
            assert_eq!(seen, expected, "{threads} threads");
        }
    }

    #[test]
    fn for_each_ordered_stops_starting_new_items_but_hands_over_all_started_ones() {
        let items: Vec<usize> = (0..200).collect();
        // One thread: stops at once, exactly like a loop with a break.
        let mut seen = Vec::new();
        super::for_each_ordered(
            &items,
            1,
            |i, _| i,
            |i, _| {
                seen.push(i);
                i < 5
            },
        );
        assert_eq!(seen, vec![0, 1, 2, 3, 4, 5]);

        // Several threads: what was started is a prefix, all of it handed
        // over in order, and far from everything was started.
        let started = std::sync::atomic::AtomicUsize::new(0);
        let mut seen = Vec::new();
        super::for_each_ordered(
            &items,
            4,
            |i, _| {
                started.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_millis(2));
                i
            },
            |i, _| {
                seen.push(i);
                i < 3
            },
        );
        let expected: Vec<usize> = (0..seen.len()).collect();
        assert_eq!(seen, expected, "a prefix, in order");
        assert_eq!(
            seen.len(),
            started.load(std::sync::atomic::Ordering::Relaxed)
        );
        assert!(
            seen.len() >= 4 && seen.len() < 100,
            "{} started",
            seen.len()
        );
    }

    #[test]
    fn an_atomic_write_creates_and_replaces_a_file_and_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.bin");
        super::write_atomic(&path, b"first").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        super::write_atomic(&path, b"second, longer").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second, longer");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(
            names,
            vec![std::ffi::OsString::from("out.bin")],
            "no temporary file"
        );
    }

    #[test]
    fn a_failed_atomic_write_keeps_the_old_file_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        // The target is a folder: the rename cannot replace it.
        let target = dir.path().join("taken");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("inside"), b"x").unwrap();
        assert!(super::write_atomic(&target, b"data").is_err());
        assert!(target.join("inside").exists());
        let leftovers = std::fs::read_dir(dir.path())
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")
            })
            .count();
        assert_eq!(leftovers, 0);
        // And a missing folder is an error, not a panic.
        assert!(super::write_atomic(&dir.path().join("nope").join("f"), b"x").is_err());
    }

    use super::*;
    use proptest::prelude::*;

    #[test]
    fn commentary_goes_to_stdout_to_stderr_or_nowhere() {
        assert_eq!(sink_for(false, false), Sink::Stdout);
        assert_eq!(
            sink_for(false, true),
            Sink::Stderr,
            "stdout carries the icon"
        );
        assert_eq!(sink_for(true, false), Sink::Nowhere);
        assert_eq!(sink_for(true, true), Sink::Nowhere, "--quiet wins");
    }

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

    // --- parse_seed ------------------------------------------------------------

    #[test]
    fn parse_seed_accepts_a_plain_pair() {
        assert_eq!(parse_seed("200,50"), Ok((200, 50)));
        assert_eq!(parse_seed("0,0"), Ok((0, 0)));
    }

    #[test]
    fn parse_seed_trims_whitespace_around_the_numbers() {
        assert_eq!(parse_seed(" 10 , 20 "), Ok((10, 20)));
    }

    #[test]
    fn parse_seed_rejects_the_wrong_number_of_parts() {
        assert!(parse_seed("").is_err());
        assert!(parse_seed("5").is_err());
        assert!(parse_seed("1,2,3").is_err());
        assert!(parse_seed("1,").is_err());
        assert!(parse_seed(",2").is_err());
    }

    #[test]
    fn parse_seed_rejects_non_numeric_and_negative_values() {
        assert!(parse_seed("a,1").is_err());
        assert!(parse_seed("1,b").is_err());
        assert!(parse_seed("-1,2").is_err());
        assert!(parse_seed("1.5,2").is_err());
    }

    #[test]
    fn parse_seed_rejects_values_beyond_u32() {
        assert!(parse_seed("4294967296,0").is_err());
    }

    #[test]
    fn parse_seed_error_names_the_bad_coordinate() {
        assert!(parse_seed("x,1").unwrap_err().contains("x coordinate"));
        assert!(parse_seed("1,y").unwrap_err().contains("y coordinate"));
    }

    // --- check_overwrite -------------------------------------------------------

    #[test]
    fn overwrite_check_passes_for_a_path_that_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(check_overwrite(&dir.path().join("new.ico"), false), Ok(()));
    }

    #[test]
    fn overwrite_check_refuses_an_existing_file_and_mentions_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.ico");
        std::fs::write(&path, b"x").unwrap();
        let err = check_overwrite(&path, false).unwrap_err();
        assert!(err.contains("--force"), "unexpected message: {err}");
        assert!(err.contains("existing.ico"), "unexpected message: {err}");
    }

    #[test]
    fn overwrite_check_allows_an_existing_file_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.ico");
        std::fs::write(&path, b"x").unwrap();
        assert_eq!(check_overwrite(&path, true), Ok(()));
    }

    // --- paths_refer_to_same_file ----------------------------------------------

    #[test]
    fn same_file_is_recognized_through_different_spellings() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.png");
        std::fs::write(&file, b"x").unwrap();
        let roundabout = dir.path().join(".").join("a.png");
        assert!(same_file(&file, &roundabout));
    }

    #[test]
    fn different_files_are_not_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        assert!(!same_file(&a, &b));
    }

    #[test]
    fn nonexistent_paths_fall_back_to_plain_comparison() {
        let a = Path::new("does/not/exist.png");
        assert!(same_file(a, a));
        assert!(!same_file(a, Path::new("does/not/exist2.png")));
    }

    // --- delete_source_files ---------------------------------------------------

    #[test]
    fn delete_source_removes_the_given_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        delete_source_files(&[a.clone(), b.clone()], None, true);
        assert!(!a.exists() && !b.exists());
    }

    #[test]
    fn delete_source_never_deletes_the_output_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("in.png");
        let output = dir.path().join("out.ico");
        std::fs::write(&source, b"x").unwrap();
        std::fs::write(&output, b"x").unwrap();
        // The output is listed among the "sources" (as happens for --merge
        // when -o points at one of the inputs).
        delete_source_files(&[source.clone(), output.clone()], Some(&output), true);
        assert!(!source.exists());
        assert!(output.exists(), "the output file must be protected");
    }

    #[test]
    fn delete_source_survives_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone.png");
        let present = dir.path().join("here.png");
        std::fs::write(&present, b"x").unwrap();
        delete_source_files(&[missing, present.clone()], None, true);
        assert!(
            !present.exists(),
            "the failure on one file must not stop the others"
        );
    }

    proptest! {
        /// parse_seed must return a normal Ok/Err for absolutely any
        /// string - never panic - for the same reason as
        /// chroma_key::parse_hex_color: it takes raw command-line text
        /// (--seed) with no restrictions on what it can contain.
        #[test]
        fn parse_seed_never_panics(input in ".*") {
            let _ = parse_seed(&input);
        }

        /// Round-trip property: any coordinate pair, formatted as "x,y"
        /// the way --seed expects it, must parse back to exactly that
        /// same pair.
        #[test]
        fn parse_seed_roundtrip(x: u32, y: u32) {
            let input = format!("{x},{y}");
            prop_assert_eq!(parse_seed(&input), Ok((x, y)));
        }
    }
}
