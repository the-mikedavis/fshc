use super::*;

use windows_sys::Win32::{
    Foundation::{STATUS_INFO_LENGTH_MISMATCH, STATUS_SUCCESS, UNICODE_STRING},
    System::WindowsProgramming::{
        // https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntquerysysteminformation
        NtQuerySystemInformation as nt_query_system_information,
        SYSTEM_INFORMATION_CLASS,
    },
};

// Usually we would use a `&'static str` to hold these values but
// `UNICODE_STRING` is UTF16 while `str` is UTF32. These are the same
// as `"<name>".to_string().encode_utf16().collect().as_slice()`.

/// `"File"` encoded as UTF16.
// const FILE_HANDLE_NAME: &'static [u16; 4] = &[70, 105, 108, 101];
/// `"ALPC Port"` encoded as UTF16.
// const ALPC_PORT_HANDLE_NAME: &'static [u16; 9] = &[65, 76, 80, 67, 32, 80, 111, 114, 116];

/// A system information class value that retrieves all handles
/// from the kernel.
const SYSTEM_HANDLE_INFORMATION: SYSTEM_INFORMATION_CLASS = 0x10;
const SYSTEM_HANDLE_INFO_BUFFER_SIZE: usize = 262144; // 2^18

const FILE_HANDLE_OBJECT_TYPE: u8 = 37;
const SOCKET_HANDLE_OBJECT_TYPE: u8 = 46;

/// An object information class that retrieves the name of the
/// kernel object's type.
// const OBJECT_TYPE_INFORMATION: OBJECT_INFORMATION_CLASS = 0x2;
// const OBJECT_INFO_BUFFER_SIZE: usize = 4096; // 2^12

// See <https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types>
type ULONG = u32;
type USHORT = u16;
type UCHAR = u8;
type PVOID = *mut std::ffi::c_void;

/// Information about a kernel object handle.
/// See <https://www.geoffchappell.com/studies/windows/km/ntoskrnl/api/ex/sysinfo/handle_table_entry.htm>
#[repr(C)]
#[derive(Debug)]
struct SystemHandleTableEntryInfo {
    unique_process_id: USHORT,
    creator_back_trace_index: USHORT,
    object_type_index: UCHAR,
    handle_attributes: UCHAR,
    handle: USHORT,
    object: PVOID,
    granted_access: ULONG,
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

/// The name of the kind of a kernel object handle.
/// See <https://www.geoffchappell.com/studies/windows/km/ntoskrnl/inc/api/ntobapi/object_type_information.htm>
// #[repr(C)]
// struct ObjectTypeInformation {
//     type_name: UNICODE_STRING,
//     reserved: [ULONG; 22],
// }

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
                    buffer.len() as u32,
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

        let pid = pid.get() as u16;
        // let pid = unsafe { GetCurrentProcessId() } as u16;
        for handle in handles {
            if handle.unique_process_id == pid {
                match handle.object_type_index {
                    FILE_HANDLE_OBJECT_TYPE => stats.file_descriptors += 1,
                    SOCKET_HANDLE_OBJECT_TYPE => stats.socket_descriptors += 1,
                    _ => (),
                }
                // let buffer = [0; OBJECT_INFO_BUFFER_SIZE].as_mut_ptr();
                // let mut return_length: ULONG = 0;
                // TODO: this needs to be executed within that process's address space.
                // if unsafe {
                //     nt_query_object(
                //         handle.handle as HANDLE,
                //         OBJECT_TYPE_INFORMATION,
                //         buffer as PVOID,
                //         OBJECT_INFO_BUFFER_SIZE as u32,
                //         &mut return_length,
                //     )
                // } == STATUS_SUCCESS
                // {
                //     let name_units = unsafe {
                //         let name = (&*(buffer as *const ObjectTypeInformation)).type_name;
                //         std::slice::from_raw_parts(name.Buffer, name.Length as usize)
                //     };

                //     if is_name_equal(FILE_HANDLE_NAME, name_units) {
                //         stats.file_descriptors += 1;
                //     } else if is_name_equal(ALPC_PORT_HANDLE_NAME, name_units) {
                //         stats.socket_descriptors += 1;
                //     }
                //     let type_name = String::from_utf16_lossy(name_units);
                //     eprintln!("handle: {} ({})", type_name, handle.object_type_index);
                // }
            }
        }

        Ok(stats)
    }
}

// fn is_name_equal(a: &[u16], b: &[u16]) -> bool {
//     b.len() >= a.len() && &b[0..a.len()] == a
// }
