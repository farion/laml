//! Resource limits apply only inside the isolated compiler, never the desktop.
#[cfg(windows)]
mod windows;
pub(super) fn apply() -> Result<(), String> {
    #[cfg(windows)]
    windows::apply()?;
    #[cfg(unix)]
    // SAFETY: pointers reference initialized rlimit/rusage structs. This process
    // only lowers its own memory limit and adjusts its own CPU soft deadline.
    unsafe {
        let mut memory: libc::rlimit = std::mem::zeroed();
        if libc::getrlimit(libc::RLIMIT_AS, &mut memory) != 0 {
            return Err("Could not inspect compiler address-space limit".into());
        }
        memory.rlim_cur = memory.rlim_cur.min(1536 * 1024 * 1024);
        memory.rlim_max = memory.rlim_max.min(1536 * 1024 * 1024);
        if libc::setrlimit(libc::RLIMIT_AS, &memory) != 0 {
            return Err("Could not limit compiler address space".into());
        }
        let mut usage: libc::rusage = std::mem::zeroed();
        let mut cpu: libc::rlimit = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0
            || libc::getrlimit(libc::RLIMIT_CPU, &mut cpu) != 0
        {
            return Err("Could not inspect compiler resource limits".into());
        }
        cpu.rlim_cur = ((usage.ru_utime.tv_sec + usage.ru_stime.tv_sec) as libc::rlim_t + 30)
            .min(cpu.rlim_max);
        if libc::setrlimit(libc::RLIMIT_CPU, &cpu) != 0 {
            return Err("Could not limit compiler CPU time".into());
        }
    }
    Ok(())
}
