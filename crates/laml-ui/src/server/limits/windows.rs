//! Job object memory containment complements the process-local wall watchdog.
use std::{
    os::windows::io::{FromRawHandle, OwnedHandle},
    sync::OnceLock,
};
use windows_sys::Win32::System::{JobObjects::*, Threading::GetCurrentProcess};
static JOB: OnceLock<Result<OwnedHandle, String>> = OnceLock::new();
pub(super) fn apply() -> Result<(), String> {
    JOB.get_or_init(|| {
        // SAFETY: the unnamed job is owned by this process, struct sizes match
        // the selected Windows ABI, and the current-process pseudo-handle lives
        // for this process's lifetime. Failure never falls back to unbounded work.
        unsafe {
            let raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if raw.is_null() {
                return Err("Could not create compiler resource job".into());
            }
            let job = OwnedHandle::from_raw_handle(raw);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            info.ProcessMemoryLimit = 1536 * 1024 * 1024;
            if SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(raw, GetCurrentProcess()) == 0
            {
                return Err("Could not constrain compiler memory with a Windows job".into());
            }
            Ok(job)
        }
    })
    .as_ref()
    .map(|_| ())
    .map_err(Clone::clone)
}
