/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

pub mod vm;
pub mod cpu;
pub mod cpu_local;

use crate::arch::task::TaskMachineContext;

//use alloc::string::String;

#[allow(unused)]
pub struct Task {
    /// A unique task identifier, there should be no other existing task with
    /// the same TID at the same time. TIDs can be reused after a task
    /// completed. Zero is not a valid TID.
    tid: u32,

    /// Tasks can be grouped into processes, the `pid` contains the task ID
    /// (TID) of the "master" task of the process; each thread of a given
    /// process thus share the same `pid` value.
    ///
    /// `pid` is equal to zero (0) for kernel threads.
    pid: u32,

    /// The parent PID of the process this task belongs to. Zero means there is
    /// no parent; only kernel threads and `userd` (pid=1) are allowed to not
    /// have a parent.
    parent_pid: u32,

    /// A descriptive name for the task, this is usually the program's name.
    //name: String,

    /// The current state of the state, whether it is running, waiting to be
    /// scheduled, waiting for an external event, suspended, etc.
    state: TaskState,

    /// The saved execution machine context used for context switching; it
    /// contains an exhaustive description of all the states to be saved and
    /// restored when switching between tasks, typically CPU registers. The
    /// exact content of this struct is arch-specific, and only arch-specific
    /// code is allowed to handle its internals.
    machine_ctx: TaskMachineContext,
    vm: vm::VirtualMemory,
    priority: i32,
}

#[allow(unused)]
pub enum TaskState {
    /// This task is currently running on a CPU.
    Running,

    /// This task is not currently running on a CPU but is ready and willing to,
    /// but instead is waiting to be scheduled for execution; this may be
    /// because there are more tasks asking for execution than there are
    /// processors available. In this state, the task is within the scheduler's
    /// runnable list.
    Runnable,

    /// The task is not currently executing on a CPU, nor is it ready to run
    /// since it is waiting for an external event to happen, or a condition to
    /// be fulfilled; this can be a point in time (e.g. a `sleep()`), the
    /// availability of a lock (sleeping for a mutex), an I/O operation, etc.
    /// It is the responsibility of the kernel to make sure the task's state is
    /// updated to `Runnable` whenever the event occurs by subscribing to it.
    Waiting,

    /// The task has been suspended, it is not running and will not run until
    /// it is resumed.
    Suspended,

    /// The task completed execution and died, but still exist as long as its
    /// parent has not read the completion status.
    Zombie,
}
