#![allow(dead_code)]
 
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SpinMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}
 
// SAFETY: access to `data` is serialised by `locked`.
unsafe impl<T: Send> Sync for SpinMutex<T> {}
unsafe impl<T: Send> Send for SpinMutex<T> {}
 
impl<T> SpinMutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }
 
    /// Acquire the lock, spinning until it is free.
    pub fn lock(&self) -> SpinGuard<'_, T> {
        let irq_state = irq_disable();
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
        SpinGuard {
            lock: self,
            irq_state,
        }
    }
 
    /// Acquire the lock only if it is uncontended. Useful in a panic handler,
    /// where blocking forever is worse than garbled output.
    pub fn try_lock(&self) -> Option<SpinGuard<'_, T>> {
        let irq_state = irq_disable();
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinGuard {
                lock: self,
                irq_state,
            })
        } else {
            irq_restore(irq_state);
            None
        }
    }
 
    /// Break the lock open.
    ///
    /// # Safety
    ///
    /// Only for a panic handler that has given up on the rest of the system.
    /// Any live guard elsewhere is now aliasing.
    pub unsafe fn force_unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}
 
pub struct SpinGuard<'a, T> {
    lock: &'a SpinMutex<T>,
    irq_state: u64,
}
 
impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: we hold the lock.
        unsafe { &*self.lock.data.get() }
    }
}
 
impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: we hold the lock exclusively.
        unsafe { &mut *self.lock.data.get() }
    }
}
 
impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        irq_restore(self.irq_state);
    }
}
 
/// Mask IRQ + FIQ, returning the previous `DAIF` so it can be restored.
#[cfg(target_arch = "aarch64")]
#[inline]
fn irq_disable() -> u64 {
    let daif: u64;
    // SAFETY: reads a system register and sets mask bits; no memory touched.
    unsafe {
        core::arch::asm!("mrs {}, daif", out(reg) daif, options(nomem, nostack));
        core::arch::asm!("msr daifset, #3", options(nomem, nostack));
    }
    daif
}
 
#[cfg(target_arch = "aarch64")]
#[inline]
fn irq_restore(daif: u64) {
    // SAFETY: restores a previously saved DAIF value.
    unsafe {
        core::arch::asm!("msr daif, {}", in(reg) daif, options(nomem, nostack));
    }
}
 
/// Stubs so the module still builds for host-side unit tests.
#[cfg(not(target_arch = "aarch64"))]
#[inline]
fn irq_disable() -> u64 {
    0
}
 
#[cfg(not(target_arch = "aarch64"))]
#[inline]
fn irq_restore(_daif: u64) {}
