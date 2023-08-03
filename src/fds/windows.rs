use super::*;

use windows_sys::Win32::{
    Foundation::FALSE,
    System::Threading::{
        // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesshandlecount
        GetProcessHandleCount as get_process_handle_count,
        // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess
        OpenProcess as open_process,
        // https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights
        PROCESS_QUERY_INFORMATION,
    },
};

impl FdList {
    pub fn list(pid: Pid) -> Result<ProcStats, FshcError> {
        let mut stats = ProcStats::new(pid);

        let handle = unsafe {
            open_process(
                PROCESS_QUERY_INFORMATION,
                // Whether child processes of this process should inherit the handle.
                FALSE,
                pid.into(),
            )
        };

        if unsafe { get_process_handle_count(handle, &mut stats.file_descriptors) } == FALSE {
            // https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
            todo!("get the last error");
        }

        Ok(stats)
    }
}
