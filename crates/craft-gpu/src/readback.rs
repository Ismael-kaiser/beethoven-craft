//! Bounded, exactly-once readback completion, independent of GPU callbacks.
// Browser callbacks stay on their worker thread. The workspace's wgpu feature still
// requires Send callbacks on non-atomic wasm, and ParticleSim requires Send + Sync.
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Condvar;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) type Error = String;
type Done = Box<dyn FnOnce(Result<Vec<u8>, Error>) + Send>;
const TIMED_OUT: &str = "GPU readback timed out; context retired";

struct Entry {
    deadline: web_time::Instant,
    done: Done,
}
#[derive(Default)]
struct State {
    next: u64,
    pending: BTreeMap<u64, Entry>,
    retired: Option<Error>,
    #[cfg(target_arch = "wasm32")]
    timer_armed: bool,
    /// The deadline worker is to exit: the context was dropped.
    #[cfg(not(target_arch = "wasm32"))]
    closed: bool,
    /// How many times the deadline worker woke up (tests: it must not wake while idle).
    #[cfg(all(test, not(target_arch = "wasm32")))]
    worker_wakes: u64,
}
/// The state and, natively, the condition the deadline worker sleeps on. The worker owns an
/// `Arc` of this, not of the [`Readbacks`], so it can block on the condition without keeping
/// the context alive; dropping the context closes it.
#[derive(Default)]
struct Inner {
    state: Mutex<State>,
    #[cfg(not(target_arch = "wasm32"))]
    wake: Condvar,
}
#[derive(Default)]
pub(crate) struct Readbacks(Arc<Inner>);

/// Retire the context when a deadline has passed: the entries to fail, or `None` when none
/// is due (or it is retired already). Decides and drains under the lock the caller holds.
fn expire_locked(s: &mut State, now: web_time::Instant) -> Option<Vec<Entry>> {
    // Decide and drain atomically: a cancelled old deadline must not retire
    // a newer request inserted between the deadline check and retirement.
    if s.retired.is_some() || !s.pending.values().any(|e| now >= e.deadline) {
        return None;
    }
    s.retired = Some(TIMED_OUT.to_string());
    Some(std::mem::take(&mut s.pending).into_values().collect())
}

/// The deadline worker. It sleeps while nothing is pending, and otherwise exactly until the
/// earliest deadline or until what is pending changes, so an idle context costs no wakeups
/// (a 100 ms polling loop here was ten wakeups a second for the life of the app). When a
/// deadline has passed it retires the context and fails every pending readback.
#[cfg(not(target_arch = "wasm32"))]
fn run_deadlines(inner: &Inner) {
    let mut s = inner.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    loop {
        if s.closed {
            return;
        }
        let now = web_time::Instant::now();
        match s.pending.values().map(|e| e.deadline).min() {
            None => s = inner.wake.wait(s).unwrap_or_else(std::sync::PoisonError::into_inner),
            Some(deadline) if deadline > now => {
                s = inner.wake.wait_timeout(s, deadline - now).unwrap_or_else(std::sync::PoisonError::into_inner).0;
            }
            Some(_) => {
                let expired = expire_locked(&mut s, now);
                drop(s);
                for entry in expired.into_iter().flatten() {
                    notify(entry.done, Err(TIMED_OUT.into()));
                }
                s = inner.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                continue;
            }
        }
        #[cfg(test)]
        {
            s.worker_wakes += 1;
        }
    }
}
pub(crate) struct Ticket {
    owner: Weak<Readbacks>,
    id: u64,
}

/// Tracks one GPU buffer mapping shared by the callback and its waiter. A timeout can race a
/// late callback, so the buffer must be unmapped exactly once. A failed map is never unmapped.
pub(crate) struct Mapping(AtomicU8);

impl Mapping {
    const PENDING: u8 = 0;
    const MAPPED: u8 = 1;
    const FAILED: u8 = 2;
    const FINISHED: u8 = 3;

    pub(crate) fn new() -> Self {
        Self(AtomicU8::new(Self::PENDING))
    }

    /// Claim a successful map callback. False means a timeout already cancelled the map.
    pub(crate) fn mapped(&self) -> bool {
        self.0.compare_exchange(Self::PENDING, Self::MAPPED, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }

    /// Record a failed map so the waiter will not unmap a buffer that was never mapped.
    pub(crate) fn failed(&self) {
        let _ = self.0.compare_exchange(Self::PENDING, Self::FAILED, Ordering::AcqRel, Ordering::Acquire);
    }

    /// Unmap once if mapping is pending (cancel it) or mapped (release it).
    pub(crate) fn unmap(&self, unmap: impl FnOnce()) {
        let mut unmap = Some(unmap);
        loop {
            let state = self.0.load(Ordering::Acquire);
            if state != Self::PENDING && state != Self::MAPPED {
                return;
            }
            if self.0.compare_exchange(state, Self::FINISHED, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                if let Some(unmap) = unmap.take() {
                    unmap();
                }
                return;
            }
        }
    }
}

/// User callbacks run outside locks and are isolated one at a time. On panic=abort
/// (including the usual wasm build), Rust panics cannot be caught: the host must
/// avoid panicking callbacks. This branch does not claim abort-mode recovery.
fn notify(done: Done, result: Result<Vec<u8>, Error>) {
    #[cfg(panic = "unwind")]
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| done(result))).is_err() {
        log::error!("GPU readback callback panicked; remaining callbacks will still settle");
    }
    #[cfg(not(panic = "unwind"))]
    done(result);
}

/// Reject an optional-pixel callback through the same isolation boundary as mapped results.
pub(crate) fn reject(done: impl FnOnce(Option<Vec<u8>>) + wgpu::WasmNotSend + 'static, error: Error) {
    notify(Box::new(move |result| done(result.ok())), Err(error));
}

impl Readbacks {
    pub(crate) fn new() -> Result<Arc<Self>, Error> {
        let me = Arc::new(Self::default());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let inner = Arc::clone(&me.0);
            std::thread::Builder::new()
                .name("gpu-readback-deadlines".into())
                .spawn(move || run_deadlines(&inner))
                .map_err(|e| format!("starting GPU readback deadline worker: {e}"))?;
        }
        Ok(me)
    }

    /// Have the deadline worker look at what is pending again (natively; the browser's timer
    /// re-arms itself).
    fn poke(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.0.wake.notify_one();
    }

    pub(crate) fn check(&self) -> Result<(), Error> {
        let s = self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match &s.retired {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    pub(crate) fn start(self: &Arc<Self>, done: impl FnOnce(Result<Vec<u8>, Error>) + wgpu::WasmNotSend + 'static) -> Result<Ticket, Error> {
        let mut done: Option<Done> = Some(Box::new(done));
        let result = {
            let mut s = self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(e) = &s.retired {
                Err(e.clone())
            } else if s.pending.len() >= 256 {
                Err("too many pending GPU readbacks".into())
            } else if let Some(id) = s.next.checked_add(1) {
                s.next = id;
                if let Some(done) = done.take() {
                    s.pending.insert(id, Entry { deadline: web_time::Instant::now() + TIMEOUT, done });
                }
                Ok(Ticket { owner: Arc::downgrade(self), id })
            } else {
                Err("GPU readback request id exhausted".into())
            }
        };
        if let Err(e) = &result
            && let Some(done) = done
        {
            notify(done, Err(e.clone()));
        }
        if result.is_ok() {
            // The worker may be asleep with nothing pending: this is its new earliest deadline.
            self.poke();
            #[cfg(target_arch = "wasm32")]
            self.arm_timer();
        }
        result
    }

    #[cfg(target_arch = "wasm32")]
    fn arm_timer(self: &Arc<Self>) {
        let delay = {
            let mut s = self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if s.timer_armed || s.retired.is_some() {
                return;
            }
            let Some(deadline) = s.pending.values().map(|e| e.deadline).min() else {
                return;
            };
            s.timer_armed = true;
            deadline.saturating_duration_since(web_time::Instant::now())
        };
        if let Err(e) = schedule(Arc::downgrade(self), delay) {
            self.retire(e);
        }
    }

    pub(crate) fn retire(&self, error: Error) {
        let pending = {
            let mut s = self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if s.retired.is_some() {
                return;
            }
            s.retired = Some(error.clone());
            std::mem::take(&mut s.pending)
        };
        self.poke();
        for (_, entry) in pending {
            notify(entry.done, Err(error.clone()));
        }
    }

    pub(crate) fn cancel_pending(&self, error: Error) {
        let pending = {
            let mut s = self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut s.pending)
        };
        self.poke();
        for (_, entry) in pending {
            notify(entry.done, Err(error.clone()));
        }
    }

    /// Retire the context when a deadline has passed (the browser's timer and tests; natively
    /// the deadline worker does this itself).
    #[cfg(any(test, target_arch = "wasm32"))]
    fn expire(&self, now: web_time::Instant) {
        let expired = {
            let mut s = self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            expire_locked(&mut s, now)
        };
        for entry in expired.into_iter().flatten() {
            notify(entry.done, Err(TIMED_OUT.into()));
        }
    }

    /// Move every pending deadline to `deadline` and wake the worker, as a test's stand-in for
    /// waiting out the production timeout.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    fn move_deadlines_for_test(&self, deadline: web_time::Instant) {
        {
            let mut s = self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            for entry in s.pending.values_mut() {
                entry.deadline = deadline;
            }
        }
        self.poke();
    }
}
impl Drop for Readbacks {
    fn drop(&mut self) {
        self.retire("GPU readback context dropped".into());
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).closed = true;
            self.0.wake.notify_all();
        }
    }
}

pub(crate) struct Layout {
    pub row: u32,
    pub stride: u32,
    pub size: u64,
    pub len: usize,
}
impl Layout {
    pub(crate) fn new(w: u32, h: u32, bpp: u32, limit: u64) -> Result<Self, Error> {
        if w == 0 || h == 0 || bpp == 0 {
            return Err("empty GPU texture readback".into());
        }
        let row = w.checked_mul(bpp).ok_or("GPU readback row size overflow")?;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let stride = row.checked_add(alignment - 1).map(|n| n / alignment * alignment).ok_or("GPU readback row alignment overflow")?;
        let size = u64::from(stride) * u64::from(h);
        // Bound host allocations even when the adapter advertises much larger buffers.
        if size > limit.min(1 << 30) {
            return Err("GPU readback exceeds buffer budget".into());
        }
        let len = usize::try_from(u64::from(row) * u64::from(h)).map_err(|e| format!("GPU readback host size: {e}"))?;
        Ok(Self { row, stride, size, len })
    }
}

impl Ticket {
    pub(crate) fn active(&self) -> bool {
        self.owner.upgrade().is_some_and(|me| me.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).pending.contains_key(&self.id))
    }
    pub(crate) fn complete(self, result: Result<Vec<u8>, Error>) {
        let Some(me) = self.owner.upgrade() else { return };
        let entry = me.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).pending.remove(&self.id);
        if let Some(entry) = entry {
            // Its deadline is gone: the worker sleeps until the next one, or until something new.
            me.poke();
            notify(entry.done, result);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn schedule(me: Weak<Readbacks>, delay: Duration) -> Result<(), Error> {
    use wasm_bindgen::{JsCast, closure::Closure};
    let global = js_sys::global();
    let function = js_sys::Reflect::get(&global, &"setTimeout".into())
        .map_err(|e| format!("GPU deadline timer lookup: {e:?}"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "GPU deadline timer unavailable".to_string())?;
    let callback = Closure::once(move || {
        if let Some(me) = me.upgrade() {
            me.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).timer_armed = false;
            me.expire(web_time::Instant::now());
            // If the earliest request completed, arm for the next live deadline.
            me.arm_timer();
        }
    });
    function
        .call2(&global, callback.as_ref(), &wasm_bindgen::JsValue::from_f64(delay.as_secs_f64() * 1000.0 + 1.0))
        .map_err(|e| format!("scheduling GPU readback deadline: {e:?}"))?;
    // Transfer the wrapper to JS GC rather than leaking a wrapper per readback.
    let _ = callback.into_js_value();
    Ok(())
}

/// A weak notification bridge. Shared-device hosts keep ownership of their handlers.
pub(crate) fn failure_notifier(me: &Arc<Readbacks>) -> impl Fn(String) + Send + Sync + 'static {
    #[cfg(not(target_arch = "wasm32"))]
    let fail = {
        let me = Arc::downgrade(me);
        move |error: String| {
            if let Some(me) = me.upgrade() {
                me.retire(error);
            }
        }
    };
    #[cfg(target_arch = "wasm32")]
    let fail = {
        // GPU callbacks require Send even on wasm. Capture only the index; the registry
        // (which can contain JS callbacks) stays on this worker's event-loop thread.
        let index = REGISTRIES.with(|r| {
            let mut r = r.borrow_mut();
            r.push(Arc::downgrade(me));
            r.len() - 1
        });
        move |error: String| {
            let me = REGISTRIES.with(|r| r.borrow().get(index).and_then(Weak::upgrade));
            if let Some(me) = me {
                me.retire(error);
            }
        }
    };
    fail
}

/// Replacement handlers are installed only by the successful owned-device request path.
pub(crate) fn install(device: &wgpu::Device, me: &Arc<Readbacks>) {
    let fail = Arc::new(failure_notifier(me));
    let lost = fail.clone();
    device.set_device_lost_callback(move |reason, message| lost(format!("GPU device lost ({reason:?}): {message}")));
    device.on_uncaptured_error(Arc::new(move |e| {
        let message = format!("uncaptured GPU error: {e}");
        fail(message.clone());
        #[cfg(test)]
        #[allow(clippy::panic)]
        if matches!(e, wgpu::Error::Validation { .. }) {
            panic!("{message}");
        }
        log::error!("{message}");
    }));
}
#[cfg(target_arch = "wasm32")]
thread_local! {
    static REGISTRIES: std::cell::RefCell<Vec<Weak<Readbacks>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Validate every row before publishing tightly packed bytes.
pub(crate) fn packed(view: &[u8], row: usize, stride: usize, height: usize) -> Result<Vec<u8>, Error> {
    rows(view, row, stride, height)
}

/// The `height` rows of `row` bytes each, `stride` bytes apart in the mapped `view`, as tightly
/// packed `T`s: one copy, straight from the mapped rows into a vector reserved once (no
/// zero-filled buffer first, no intermediate byte vector). Every row is validated before any
/// is copied. A row that is not aligned for `T` is decoded value by value (same bytes).
pub(crate) fn rows<T: bytemuck::Pod>(view: &[u8], row: usize, stride: usize, height: usize) -> Result<Vec<T>, Error> {
    let unit = std::mem::size_of::<T>().max(1);
    if row == 0 || height == 0 || stride < row || !row.is_multiple_of(unit) {
        return Err("invalid GPU readback layout".into());
    }
    let size = row.checked_mul(height).ok_or("GPU readback output size overflow")? / unit;
    let end = (height - 1).checked_mul(stride).and_then(|s| s.checked_add(row)).ok_or("GPU readback mapped size overflow")?;
    if view.len() < end {
        return Err(format!("short GPU readback: {} bytes, needs {end}", view.len()));
    }
    let mut out: Vec<T> = Vec::new();
    out.try_reserve_exact(size).map_err(|e| format!("allocating GPU readback output: {e}"))?;
    for y in 0..height {
        let start = y.checked_mul(stride).ok_or("GPU readback row overflow")?;
        let bytes = start.checked_add(row).and_then(|e| view.get(start..e)).ok_or("short GPU readback row")?;
        match bytemuck::try_cast_slice::<u8, T>(bytes) {
            Ok(values) => out.extend_from_slice(values),
            Err(_) => out.extend(bytes.chunks_exact(unit).map(bytemuck::pod_read_unaligned::<T>)),
        }
    }
    Ok(out)
}

