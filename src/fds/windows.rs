use super::*;

use windows_sys::Win32::{
    Foundation::{STATUS_INFO_LENGTH_MISMATCH, STATUS_SUCCESS},
    System::WindowsProgramming::{
        // https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntquerysysteminformation
        NtQuerySystemInformation as nt_query_system_information,
        SYSTEM_INFORMATION_CLASS,
    },
};

// See <https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types>
type ULONG = u32;
type USHORT = u16;
type UCHAR = u8;
type PVOID = *mut std::ffi::c_void;

// These type ID numbers can be determined by querying the object type
//
// ```rust
// TODO
// ```

/// The type ID of "File" kernel objects.
const FILE_HANDLE_OBJECT_TYPE_ID: UCHAR = 37;
/// The type ID of "ALPC Port" (socket) kernel objects.
const SOCKET_HANDLE_OBJECT_TYPE_ID: UCHAR = 46;

// The system handle information request and response are not
// officially documented so we need to write our own type wrappers.

/// A system information class value that retrieves all handles
/// from the kernel.
const SYSTEM_HANDLE_INFORMATION: SYSTEM_INFORMATION_CLASS = 0x10;
const SYSTEM_HANDLE_INFO_BUFFER_SIZE: usize = 262144; // 2^18

/// Information about a kernel object handle.
/// See <https://www.geoffchappell.com/studies/windows/km/ntoskrnl/api/ex/sysinfo/handle_table_entry.htm>
#[repr(C)]
#[derive(Debug)]
struct SystemHandleTableEntryInfo {
    /// The ID of the process which holds the handle.
    process_id: USHORT,
    _creator_back_trace_index: USHORT,
    /// The type of object described by the handle.
    /// See `FILE_HANDLE_OBJECT_TYPE_ID` and `SOCKET_HANDLE_OBJECT_TYPE_ID`.
    object_type_id: UCHAR,
    _handle_attributes: UCHAR,
    _handle: USHORT,
    _object: PVOID,
    _granted_access: ULONG,
}

/// A vector of all kernel object handles in the system.
/// See <https://www.geoffchappell.com/studies/windows/km/ntoskrnl/api/ex/sysinfo/handle.htm>
#[repr(C)]
#[derive(Debug)]
struct SystemHandleInformation {
    number_of_handles: ULONG,
    /// 1-length arrays are interpreted as any-length arrays.
    /// This value should be used as a pointer and re-cast into a slice
    /// `[SystemHandleTableEntryInfo; number_of_handles]`.
    handles: [SystemHandleTableEntryInfo; 1],
}

impl FdList {
    pub fn list(pid: Pid) -> Result<ProcStats, FshcError> {
        let mut stats = ProcStats::new(pid);

        // Get the list of all open kernel object handles.
        let mut buffer: Vec<u8> = Vec::with_capacity(SYSTEM_HANDLE_INFO_BUFFER_SIZE);
        loop {
            buffer.resize(buffer.len() + SYSTEM_HANDLE_INFO_BUFFER_SIZE, 0);
            let mut return_length: ULONG = 0;
            match unsafe {
                nt_query_system_information(
                    SYSTEM_HANDLE_INFORMATION,
                    buffer.as_mut_ptr() as PVOID,
                    buffer.len() as ULONG,
                    &mut return_length,
                )
            } {
                // We can't query the size of the list so we query
                // repeatedly, increasing the size of the input buffer
                // linearly on each iteration.
                STATUS_INFO_LENGTH_MISMATCH => continue,
                STATUS_SUCCESS => break,
                // TODO
                other => unimplemented!("Unknown error code: {other}"),
            }
        }
        let handles = unsafe {
            let info = &*(buffer.as_ptr() as *const SystemHandleInformation);
            std::slice::from_raw_parts(info.handles.as_ptr(), info.number_of_handles as usize)
        };

        // Count file and socket object handles belonging to the given
        // process.
        let pid = pid.get() as u16;
        for handle in handles {
            if handle.process_id == pid {
                match handle.object_type_id {
                    FILE_HANDLE_OBJECT_TYPE_ID => stats.file_descriptors += 1,
                    SOCKET_HANDLE_OBJECT_TYPE_ID => stats.socket_descriptors += 1,
                    _ => (),
                }
            }
        }

        Ok(stats)
    }
}
