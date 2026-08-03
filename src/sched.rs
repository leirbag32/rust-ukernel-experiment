use crate::arch::aarch64::context::{cpu_switch_to, Context};

pub const MAX_TASKS: usize = 8;
pub const STACK_SIZE: usize = 16 * 1024;

pub type TaskId = usize;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Unused,
    Ready,
    Running,
    Blocked,
    Exited,
}

// Simple Task for now
pub struct Task {
    pub state: State,
    pub ctx: Context,
}

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

// Pre-allocate all the stacks, it's not memory efficient but that avoids dynamic memory allocation
static mut STACKS: [Stack; MAX_TASKS] = unsafe { core::mem::zeroed() };

// Sched representation, basically a context + ID
struct Sched {
    tasks: [Task; MAX_TASKS],
    current: TaskId,
}

static mut SCHED: Sched = Sched {
    tasks: [const {
        Task {
            state: State::Unused,
            ctx: Context {
                x19: 0, x20: 0, x21: 0, x22: 0, x23: 0,
                x24: 0, x25: 0, x26: 0, x27: 0, x28: 0,
                fp: 0, lr: 0, sp: 0,
            },
        }
    }; MAX_TASKS],
    current: 0,
};

// Get a pointer to the current task running
fn sched() -> &'static mut Sched {
    unsafe { &mut *core::ptr::addr_of_mut!(SCHED) }
}

pub fn init() {
    // Task ID=0 is always the kernel main thread
    let s = sched();
    s.tasks[0].state = State::Running;
    s.current = 0;
}

// Make sure to take a "extern C" to enforce stable ABI to save/restore the registers
pub fn spawn(entry: extern "C" fn()) -> Option<TaskId> {
    let s = sched();
    let id = s.tasks.iter().position(|t| t.state == State::Unused)?;
    let stack_top = unsafe {
        core::ptr::addr_of!(STACKS[id]) as u64 + STACK_SIZE as u64
    };
    s.tasks[id].ctx = Context::new(entry, stack_top);
    s.tasks[id].state = State::Ready;
    Some(id)
}

// Get the current ID to the task running
pub fn current() -> TaskId {
    sched().current
}

// Schedule the new task
pub fn yield_now() {
    let s = sched();
    let cur = s.current;

    let mut next = None;
    for off in 1..=MAX_TASKS {
        let cand = (cur + off) % MAX_TASKS;
        if s.tasks[cand].state == State::Ready {
            next = Some(cand);
            break;
        }
    }
    let Some(next) = next else { return };

    if s.tasks[cur].state == State::Running {
        s.tasks[cur].state = State::Ready;
    }
    s.tasks[next].state = State::Running;
    s.current = next;

    let prev_ctx: *mut Context = &mut s.tasks[cur].ctx;
    let next_ctx: *const Context = &s.tasks[next].ctx;
    unsafe { cpu_switch_to(prev_ctx, next_ctx) };
}

// Return all the tasks that are either ready/running or blocked
pub fn alive_tasks() -> usize {
    sched()
        .tasks
        .iter()
        .filter(|t| matches!(t.state, State::Ready | State::Running | State::Blocked))
        .count()
}

#[no_mangle]
extern "C" fn task_exit() -> ! {
    let s = sched();
    let cur = s.current;
    s.tasks[cur].state = State::Exited;
    loop {
        yield_now();
        // If we're ever resumed with nothing to run, just wait.
        crate::arch::wait_for_event();
    }
}
