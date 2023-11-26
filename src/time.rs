use std::{sync::atomic::AtomicU64, thread::spawn, time::Duration};

static TIMER: AtomicU64 = AtomicU64::new(0);
const INCREMENT: u64 = 100;

pub fn get_time() -> u64 {
    TIMER.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn update_time() -> u64 {
    TIMER.fetch_add(INCREMENT, std::sync::atomic::Ordering::SeqCst)
}

pub fn start_timer() {
    spawn(|| loop {
        update_time();
        std::thread::sleep(Duration::from_millis(INCREMENT));
    });
}
