// Splitting work over threads, where threads exist. With one thread (or one
// item) nothing is spawned, which is also how the web version runs it: a
// browser page has no threads to hand out.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Runs `work` for every item - on up to `threads` threads - and returns the
/// results in the order of `items`, whatever order they finished in. With one
/// thread (or one item) nothing is spawned and it is a plain loop.
pub fn parallel_map<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    work: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_map_keeps_the_order_of_the_items() {
        let items: Vec<u32> = (0..50).collect();
        for threads in [1, 2, 8, 100] {
            let squares = parallel_map(&items, threads, |&n| {
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
        assert!(parallel_map(&Vec::<u32>::new(), 4, |&n| n).is_empty());
    }

    #[test]
    fn parallel_map_really_uses_several_threads() {
        let ids = parallel_map(&[0; 8], 4, |_| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            std::thread::current().id()
        });
        let distinct: std::collections::HashSet<_> = ids.into_iter().collect();
        assert!(distinct.len() > 1, "all on one thread: {distinct:?}");
    }
}
