//! A token bucket with a queue that requests can leave (ADR 0016).
//!
//! Tokens refill at a steady rate up to a small burst. Waiting requests are ordered, not
//! first-come: metadata before images, metadata oldest first, images newest first, and only one
//! image in flight. A waiter that is dropped leaves the queue without taking a token, so a caller
//! cancels a request by dropping its future, and an expired one by wrapping it in a timeout.
//!
//! With a [`SharedBucket`], the tokens come from a file that other processes share, so separate
//! CLI runs and the TUI keep to one limit together. The queue itself stays per process.

use std::{
    pin::pin,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::{sync::Notify, time::Instant};

use super::shared::SharedBucket;

/// Which queue a request waits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lane {
    /// Searches, listings and records: served first, oldest first.
    Metadata,
    /// Images: served after metadata, newest first, one in flight at a time.
    Image,
}

/// Shared by every clone of a `Catalog`.
#[derive(Debug)]
pub(crate) struct Limiter {
    state: Mutex<State>,
    /// Woken whenever the head of the queue may have changed.
    changed: Notify,
    /// Time to earn one token.
    interval: Duration,
    /// Most tokens the bucket holds.
    burst: f64,
    /// Where tokens come from when they are shared with other processes.
    shared: Option<SharedBucket>,
    /// Whether a failure of the shared bucket has been logged, so it is logged once.
    shared_failed: AtomicBool,
    /// Whether the head of the queue is waiting for a token right now.
    starved: AtomicBool,
}

#[derive(Debug)]
struct State {
    tokens: f64,
    refilled: Instant,
    next_ticket: u64,
    waiting: Vec<Waiter>,
    images_in_flight: usize,
}

#[derive(Debug, Clone, Copy)]
struct Waiter {
    ticket: u64,
    lane: Lane,
}

impl State {
    /// Adds the tokens earned since the last refill.
    fn refill(&mut self, now: Instant, interval: Duration, burst: f64) {
        let earned = now.duration_since(self.refilled).as_secs_f64() / interval.as_secs_f64();
        self.tokens = (self.tokens + earned).min(burst);
        self.refilled = now;
    }

    /// The ticket that is served next, if anyone may be served.
    fn head(&self) -> Option<u64> {
        let metadata = self
            .waiting
            .iter()
            .filter(|w| w.lane == Lane::Metadata)
            .map(|w| w.ticket)
            .min();
        if metadata.is_some() {
            return metadata;
        }
        if self.images_in_flight > 0 {
            return None;
        }
        self.waiting
            .iter()
            .filter(|w| w.lane == Lane::Image)
            .map(|w| w.ticket)
            .max()
    }

    fn leave(&mut self, ticket: u64) {
        self.waiting.retain(|w| w.ticket != ticket);
    }
}

impl Limiter {
    /// A bucket that earns `per_minute` tokens a minute and holds at most `burst`, starting full.
    pub(crate) fn new(per_minute: u32, burst: u32) -> Self {
        Self {
            state: Mutex::new(State {
                tokens: f64::from(burst.max(1)),
                refilled: Instant::now(),
                next_ticket: 0,
                waiting: Vec::new(),
                images_in_flight: 0,
            }),
            changed: Notify::new(),
            interval: Duration::from_secs(60) / per_minute.max(1),
            burst: f64::from(burst.max(1)),
            shared: None,
            shared_failed: AtomicBool::new(false),
            starved: AtomicBool::new(false),
        }
    }

    /// Takes tokens from `bucket`, shared with other processes, instead of from memory.
    pub(crate) fn shared(mut self, bucket: SharedBucket) -> Self {
        self.shared = Some(bucket);
        self
    }

    /// Takes a token, or returns how long until one is free.
    ///
    /// A shared bucket that can't be read or written falls back to the in-memory one, so a
    /// read-only state directory slows nothing down. It is logged once.
    fn take(&self, state: &mut State) -> Result<(), Duration> {
        if let Some(shared) = &self.shared {
            match shared.try_take(self.interval, self.burst) {
                Ok(taken) => return taken,
                Err(error) => {
                    if !self.shared_failed.swap(true, Ordering::Relaxed) {
                        tracing::warn!(%error, "shared rate limit unavailable, limiting per process");
                    }
                }
            }
        }
        state.refill(Instant::now(), self.interval, self.burst);
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            Ok(())
        } else {
            Err(self.interval.mul_f64(1.0 - state.tokens))
        }
    }

    /// Whether a request is waiting for a token now, rather than behind other requests or not
    /// at all.
    pub(crate) fn is_starved(&self) -> bool {
        self.starved.load(Ordering::Relaxed)
    }

    /// Locks the state. A panic while it was held can't leave it half-updated in a way that
    /// matters here, so a poisoned lock is used as is.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Waits for a token in `lane`.
    ///
    /// Dropping the returned future leaves the queue. Dropping the [`Permit`] ends the request,
    /// which for an image lets the next image go.
    pub(crate) async fn acquire(self: &Arc<Self>, lane: Lane) -> Permit {
        let ticket = {
            let mut state = self.lock();
            let ticket = state.next_ticket;
            state.next_ticket += 1;
            state.waiting.push(Waiter { ticket, lane });
            ticket
        };
        // Leaves the queue if this future is dropped before it is served.
        let mut place = Place {
            limiter: self,
            ticket,
            served: false,
        };

        loop {
            // Register for wake-ups *before* looking at the state, so a change made between the
            // look and the wait is not missed.
            let mut changed = pin!(self.changed.notified());
            changed.as_mut().enable();

            let next_token_at = {
                let mut state = self.lock();
                if state.head() != Some(ticket) {
                    None
                } else if let Err(wait) = self.take(&mut state) {
                    self.starved.store(true, Ordering::Relaxed);
                    Some(Instant::now() + wait)
                } else {
                    self.starved.store(false, Ordering::Relaxed);
                    state.leave(ticket);
                    if lane == Lane::Image {
                        state.images_in_flight += 1;
                    }
                    drop(state);
                    place.served = true;
                    // The head moved on; let the next waiter look.
                    self.changed.notify_waiters();
                    return Permit {
                        limiter: Arc::clone(self),
                        lane,
                    };
                }
            };

            match next_token_at {
                // At the head: sleep until the next token, unless the queue changes first.
                Some(at) => {
                    let _ = tokio::time::timeout_at(at, changed).await;
                }
                // Behind someone: wait until the queue changes.
                None => changed.await,
            }
        }
    }
}

/// A place in the queue, removed when the waiting future is dropped.
struct Place<'a> {
    limiter: &'a Limiter,
    ticket: u64,
    served: bool,
}

impl Drop for Place<'_> {
    fn drop(&mut self) {
        if self.served {
            return;
        }
        let mut state = self.limiter.lock();
        state.leave(self.ticket);
        if state.waiting.is_empty() {
            self.limiter.starved.store(false, Ordering::Relaxed);
        }
        drop(state);
        self.limiter.changed.notify_waiters();
    }
}

/// Held while a request is in flight.
#[derive(Debug)]
pub(crate) struct Permit {
    limiter: Arc<Limiter>,
    lane: Lane,
}

impl Drop for Permit {
    fn drop(&mut self) {
        if self.lane == Lane::Image {
            self.limiter.lock().images_in_flight -= 1;
            self.limiter.changed.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    /// Lets spawned tasks run up to their next wait, without moving the paused clock.
    async fn settle() {
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }

    fn since(start: Instant) -> u64 {
        start.elapsed().as_millis() as u64
    }

    #[tokio::test(start_paused = true)]
    async fn serves_a_burst_then_one_a_second() {
        let limiter = Arc::new(Limiter::new(60, 2));
        let start = Instant::now();
        let mut at = Vec::new();
        for _ in 0..4 {
            drop(limiter.acquire(Lane::Metadata).await);
            at.push(since(start));
        }
        assert_eq!(at, [0, 0, 1000, 2000]);
    }

    #[tokio::test(start_paused = true)]
    async fn reports_starved_only_while_waiting_for_a_token() {
        let limiter = Arc::new(Limiter::new(60, 1));
        let _first = limiter.acquire(Lane::Metadata).await;
        assert!(!limiter.is_starved());
        let waiter = tokio::spawn({
            let limiter = Arc::clone(&limiter);
            async move { limiter.acquire(Lane::Metadata).await }
        });
        // Make sure waiter runs ahead enough to put the limiter in starved state, without advancing
        // the clock.
        settle().await;
        assert!(limiter.is_starved(), "the bucket is empty");
        assert!(!waiter.is_finished(), "still waiting for a token");
        // Advance the paused clock forward 1 sec to refill the bucket and let the waiter complete.
        tokio::time::advance(Duration::from_secs(1)).await;
        waiter.await.unwrap();
        assert!(!limiter.is_starved(), "served");
    }

    #[tokio::test(start_paused = true)]
    async fn a_dropped_waiter_takes_no_token() {
        let limiter = Arc::new(Limiter::new(60, 1));
        drop(limiter.acquire(Lane::Metadata).await);
        let start = Instant::now();

        let early = tokio::spawn({
            let limiter = Arc::clone(&limiter);
            async move { drop(limiter.acquire(Lane::Metadata).await) }
        });
        settle().await;
        early.abort();
        settle().await;

        // The next two are served at 1 s and 2 s, as if the aborted waiter was never there.
        drop(limiter.acquire(Lane::Metadata).await);
        assert_eq!(since(start), 1000);
        drop(limiter.acquire(Lane::Metadata).await);
        assert_eq!(since(start), 2000);
    }

    #[tokio::test(start_paused = true)]
    async fn timing_out_leaves_the_queue() {
        let limiter = Arc::new(Limiter::new(60, 1));
        drop(limiter.acquire(Lane::Metadata).await);
        let start = Instant::now();

        let deadline = start + Duration::from_millis(500);
        let expired = tokio::time::timeout_at(deadline, limiter.acquire(Lane::Metadata)).await;
        assert!(expired.is_err());
        assert!(limiter.lock().waiting.is_empty());

        drop(limiter.acquire(Lane::Metadata).await);
        assert_eq!(since(start), 1000);
    }

    /// Queues one waiter per lane in order, then reports the order they were served in.
    async fn served_order(lanes: &[Lane]) -> Vec<usize> {
        let limiter = Arc::new(Limiter::new(60, 1));
        drop(limiter.acquire(Lane::Metadata).await);
        let (tx, mut rx) = mpsc::unbounded_channel();
        for (i, &lane) in lanes.iter().enumerate() {
            let limiter = Arc::clone(&limiter);
            let tx = tx.clone();
            tokio::spawn(async move {
                let permit = limiter.acquire(lane).await;
                tx.send(i).unwrap();
                drop(permit);
            });
            settle().await;
        }
        drop(tx);
        let mut order = Vec::new();
        while let Some(i) = rx.recv().await {
            order.push(i);
        }
        order
    }

    #[tokio::test(start_paused = true)]
    async fn metadata_is_served_before_images() {
        use Lane::*;
        assert_eq!(
            served_order(&[Image, Metadata, Image, Metadata]).await,
            [1, 3, 2, 0]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn images_are_served_newest_first() {
        use Lane::*;
        assert_eq!(served_order(&[Image, Image, Image]).await, [2, 1, 0]);
    }

    #[tokio::test(start_paused = true)]
    async fn one_image_in_flight_at_a_time() {
        let limiter = Arc::new(Limiter::new(60, 5));
        let first = limiter.acquire(Lane::Image).await;

        let second = tokio::spawn({
            let limiter = Arc::clone(&limiter);
            async move { limiter.acquire(Lane::Image).await }
        });
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(!second.is_finished(), "waits although tokens are free");

        // Metadata still goes while an image is in flight.
        drop(limiter.acquire(Lane::Metadata).await);

        drop(first);
        drop(second.await.unwrap());
    }
}
