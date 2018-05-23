//! This crate allows you to rougly limit the amount of time taken by a repetitive task.
//!
//! For this crate to work well, the task you're doing should take a reasonably consistent amount of time.
//!
//! ```
//! # extern crate soft_time_limit;
//! # use soft_time_limit::TimeLimiter;
//! # use std::time::Duration;
//! # struct ThingToDo(u32);
//! # let mut game_over = false;
//! let mut limit = TimeLimiter::new();
//!
//! // outer game / simulation loop:
//! while !game_over {
//!     // Each frame, we want to perform some number of time-consuming tasks,
//!     // without using up too much time. We'll give our tasks a time budget
//!     // of 3ms.
//!     limit.repeat_with_budget(Duration::from_millis(3), || {
//!         // This closure will be called repeatedly, until it returns false
//!         // or we predict that we're going to overshoot our time limit.
//!
//!         // ... do something expensive ...
//!
//!         // return whether we want to be invoked again.
//!         # let more_tasks_to_do = false;
//!         return more_tasks_to_do;
//!     });
//!     # break;
//! }
//! ```
//!
//! Another example, processing items from a queue:
//!
//! ```
//! # extern crate soft_time_limit;
//! # use soft_time_limit::TimeLimiter;
//! # use std::time::Duration;
//! # struct ThingToDo(u32);
//! # let mut game_over = false;
//! let mut queue = vec![ThingToDo(1), ThingToDo(2)];
//! let mut limit = TimeLimiter::new();
//! while !game_over {
//!     limit.repeat_with_budget(Duration::from_millis(3), || {
//!         if !queue.is_empty() {
//!             let thing_to_do = queue.pop();
//!             // ... do expensive task with thing_to_do ...
//!         }
//!         let is_done = !queue.is_empty();
//!         is_done
//!     });
//!     # break;
//! }
//! ```
//!
//! If you have multiple tasks you want to limit, you should create a *separate* `TimeLimiter` for each task.
//! That is, you should never call two different closures with the same `TimeLimiter`.
//!
//! You can also explicitly manage the time frame you have available:
//!
//! ```
//! # extern crate soft_time_limit;
//! # use soft_time_limit::TimeLimiter;
//! # use std::time::Duration;
//! # let mut game_over = false;
//! let mut limit = TimeLimiter::new();
//! while !game_over {
//!     // 5ms time limit for our task:
//!     let mut frame = limit.frame(Duration::from_millis(5));
//!     while frame.have_time() {
//!         // the time a task takes is measured from the creation to the drop of this handle
//!         // make sure you don't assign to _, it will be dropped immediately if you do
//!         let _task = frame.time_task();
//!         // ... do expensive task ...
//!         # break;
//!     }
//!     # break;
//! }
//! ```
//!
//! Note: This crate is very low-overhead and never invokes thread sleeps;
//! it just chooses whether to call your function or not based on the system time.
//! However, it won't magically make your tasks faster.
//! You'll still need to make sure they complete in a reasonable amount of time 😉

use std::time::{Duration, Instant};

/// Keeps track of the time taken by some task.
#[derive(Clone, Debug)]
pub struct TimeLimiter {
    /// A running average of the time taken by the task in the past.
    /// In units of seconds.
    pub time_estimate: f64,
    /// The proportion used in the running average:
    ///
    /// `time_estimate = task_time * smoothing + time_estimate * (1.0 - smoothing);`
    pub smoothing: f64,
    /// The decay of the time estimate used every frame:
    ///
    /// `time_estimate = time_estimate * decay;`
    ///
    /// If the time estimate didn't decay, then the system would lock forever
    /// if the time estimate went over the per-frame time budget.
    /// This way the system is guaranteed to at least run one task every few frames.
    pub decay: f64,
}
impl TimeLimiter {
    /// Create a TimeLimiter with the default averaging rates (0.1 smoothing, 0.99 decay)
    pub fn new() -> TimeLimiter {
        Default::default()
    }

    /// Create a TimeLimiter with a custom smoothing rate for the running average
    /// and a custom decay rate.
    pub fn with_rates(smoothing: f64, decay: f64) -> TimeLimiter {
        assert!(smoothing < 1.0, "smoothing too large");
        assert!(smoothing > 0.0, "smoothing too small");
        assert!(decay < 1.0, "smoothing too large");
        assert!(decay > 0.0, "smoothing too small");
        TimeLimiter {
            smoothing,
            decay,
            time_estimate: 0.0,
        }
    }

    /// Repeatedly calls a function until either:
    /// 1. The estimated time to complete the task goes over the time budget, OR
    /// 2. The function returns false.
    pub fn repeat_with_budget<F: FnMut() -> bool>(&mut self, budget: Duration, mut f: F) {
        let mut frame = self.frame(budget);

        while frame.have_time() {
            let _task = frame.time_task();
            let should_continue = f();
            if !should_continue {
                break;
            }
        }
    }

    /// Manually start timing a single frame.
    pub fn frame(&mut self, budget: Duration) -> Frame {
        self.time_estimate *= self.decay;
        Frame {
            limiter: self,
            deadline: Instant::now() + budget,
        }
    }
}

impl Default for TimeLimiter {
    fn default() -> Self {
        TimeLimiter::with_rates(0.1, 0.99)
    }
}

/// A lock representing a single frame.
pub struct Frame<'a> {
    limiter: &'a mut TimeLimiter,
    deadline: Instant,
}

impl<'a> Frame<'a> {
    /// Whether or not there's enough time available to perform one of our tasks.
    pub fn have_time(&self) -> bool {
        let result = Instant::now() + to_duration(self.limiter.time_estimate) < self.deadline;
        result
    }

    /// Create a Task; when it is dropped, we'll compute the elapsed time and update
    /// our time estimates.
    pub fn time_task<'b>(&'b mut self) -> Task<'b, 'a> {
        Task {
            frame: self,
            start: Instant::now(),
        }
    }
}

/// A lock representing a single task within a frame.
pub struct Task<'b, 'a: 'b> {
    frame: &'b mut Frame<'a>,
    start: Instant,
}

impl<'b, 'a: 'b> Drop for Task<'b, 'a> {
    fn drop(&mut self) {
        let duration = Instant::now() - self.start;
        let limiter = &mut self.frame.limiter;

        limiter.time_estimate = limiter.time_estimate * (1.0 - limiter.smoothing)
            + to_float(duration) * limiter.smoothing;
    }
}

fn to_float(duration: Duration) -> f64 {
    duration.as_secs() as f64 + 0.000_000_001 * duration.subsec_nanos() as f64
}
fn to_duration(duration: f64) -> Duration {
    Duration::new(
        duration.trunc() as u64,
        (duration.fract() * 1_000_000_000.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::TimeLimiter;
    use std::thread::sleep;
    use std::time::Duration;

    // this crate is hard to test because system timing isn't consistent :/

    #[test]
    fn timing_repeat() {
        let mut limit = TimeLimiter::new();

        let mut tasks = 0.0;

        let n = 5;

        for _ in 0..n {
            limit.repeat_with_budget(Duration::from_millis(10), || {
                sleep(Duration::from_millis(1));
                tasks += 1.0;
                true
            });
        }

        // thinking emoji
        println!(
            "timing_repeat: average tasks (should be 10 or less): {}",
            tasks / n as f32
        );
    }

    #[test]
    fn timing_explicit() {
        let mut limit = TimeLimiter::new();

        let mut tasks = 0.0;

        let n = 5;

        for _ in 0..n {
            let mut frame = limit.frame(Duration::from_millis(10));

            while frame.have_time() {
                let _task = frame.time_task();

                sleep(Duration::from_millis(1));
                tasks += 1.0;
            }
        }
        println!(
            "timing_explicit: average tasks (should be 10 or less): {}",
            tasks / n as f32
        );
    }

}
