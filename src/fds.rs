use crate::outcome::*;
#[cfg(target_os = "macos")]
use libproc::libproc::{
    bsd_info::BSDInfo,
    file_info::{ListFDs, ProcFDType},
    proc_pid::{listpidinfo, pidinfo},
};
#[cfg(target_os = "linux")]
use procfs::process::{FDTarget, Process};

pub struct FdList;

#[cfg(target_os = "macos")]
impl FdList {
    pub fn list(pid: i32) -> Result<ProcStats, FshcError> {
        let info = pidinfo::<BSDInfo>(pid, 0)?;
        let fds = listpidinfo::<ListFDs>(pid, info.pbi_nfiles as usize)?;

        let mut stats = ProcStats {
            pid,
            socket_descriptors: 0,
            file_descriptors: 0,
        };
        for fd in fds {
            // libproc returns file descriptor types as numbers,
            // try to convert them
            if let ProcFDType::Socket = fd.proc_fdtype.into() {
                stats.socket_descriptors += 1;
            }
            if let ProcFDType::VNode = fd.proc_fdtype.into() {
                stats.file_descriptors += 1;
            }
        }

        Ok(stats)
    }
}

#[cfg(target_os = "linux")]
impl FdList {
    pub fn list(pid: i32) -> Result<ProcStats, FshcError> {
        let proc = Process::new(pid)?;
        let all_fds = proc.fd()?;

        let mut stats = ProcStats {
            pid,
            socket_descriptors: 0,
            file_descriptors: 0,
        };
        let fds = all_fds
            .flatten()
            .filter(|fd_info| matches!(fd_info.target, FDTarget::Path(_) | FDTarget::Socket(_)));
        for fd in fds {
            match fd.target {
                FDTarget::Path(_) => stats.file_descriptors += 1,
                FDTarget::Socket(_) => stats.socket_descriptors += 1,
                _ => (),
            }
        }

        Ok(stats)
    }
}

#[cfg(target_os = "windows")]
impl FdList {
    pub fn list(pid: i32) -> Result<ProcStats, FshcError> {
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

        // TODO: bail if pid <= 0;

        let mut stats = ProcStats {
            pid,
            socket_descriptors: 0,
            file_descriptors: 0,
        };

        let handle = unsafe {
            open_process(
                PROCESS_QUERY_INFORMATION,
                // Whether child processes of this process should inherit the handle.
                FALSE,
                pid as i32,
            )
        };

        if unsafe { get_process_handle_count(handle, &mut stats.file_descriptors) } == FALSE {
            // https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
            todo!("get the last error");
        }

        Ok(stats)
    }
}
