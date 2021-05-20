use std::{future::Future, pin::Pin, task::Context, thread};
use waker_fn::waker_fn;

/// Runs all of the futures in the `iterator` using `futures` as
/// a temporary work buffer
pub fn run_all<I, F>(iterator: I, futures: &mut Vec<F>)
where
    I: Iterator<Item = F>,
    F: Future<Output = ()>,
{
    futures.clear();

    let thread = thread::current();
    let waker = waker_fn(move || thread.unpark());
    let cx = &mut Context::from_waker(&waker);

    futures.extend(
        iterator
            .map(|future| poll_once(future, cx))
            .filter(Option::is_some)
            .map(Option::unwrap),
    );

    while !futures.is_empty() {
        let mut i = 0;
        let mut len = futures.len();
        while i < len {
            if poll_one(&mut futures[i], cx) {
                futures.swap_remove(i);
                len -= 1;
            } else {
                i += 1;
            }
        }
    }
}

#[inline]
fn poll_one<F>(future: &mut F, cx: &mut Context) -> bool
where
    F: Future,
{
    // TODO: remove this usage to unsafe
    let pin = unsafe { Pin::new_unchecked(future) };

    pin.poll(cx).is_ready()
}

#[inline]
fn poll_once<F>(mut future: F, cx: &mut Context) -> Option<F>
where
    F: Future,
{
    // TODO: remove this usage to unsafe
    let pin = unsafe { Pin::new_unchecked(&mut future) };

    if pin.poll(cx).is_pending() {
        Some(future)
    } else {
        None
    }
}
