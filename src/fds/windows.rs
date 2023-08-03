use super::*;

use windows_sys::Win32::{
    Foundation::{HANDLE, STATUS_INFO_LENGTH_MISMATCH, STATUS_SUCCESS, UNICODE_STRING},
    System::WindowsProgramming::{
        NtQueryObject as nt_query_object, NtQuerySystemInformation as nt_query_system_information,
        SYSTEM_INFORMATION_CLASS,
    },
};

const SYSTEM_HANDLE_INFORMATION: SYSTEM_INFORMATION_CLASS = 0x10;
const STEP: usize = 16384;

type ULONG = u32;
type USHORT = u16;
// type BYTE = u8;
type UCHAR = u8;
type PVOID = *mut std::ffi::c_void;

// https://www.geoffchappell.com/studies/windows/km/ntoskrnl/api/ex/sysinfo/handle_table_entry.htm

/*
typedef struct _SYSTEM_HANDLE_TABLE_ENTRY_INFO
{
    USHORT UniqueProcessId;
    USHORT CreatorBackTraceIndex;
    UCHAR ObjectTypeIndex;
    UCHAR HandleAttributes;
    USHORT HandleValue;
    PVOID Object;
    ULONG GrantedAccess;
} SYSTEM_HANDLE_TABLE_ENTRY_INFO, *PSYSTEM_HANDLE_TABLE_ENTRY_INFO;

typedef struct _SYSTEM_HANDLE_INFORMATION
{
    ULONG NumberOfHandles;
    SYSTEM_HANDLE_TABLE_ENTRY_INFO Handles[1];
} SYSTEM_HANDLE_INFORMATION, *PSYSTEM_HANDLE_INFORMATION;
*/

#[repr(C)]
#[derive(Debug)]
struct SystemHandleTableEntryInfo {
    unique_process_id: USHORT,
    creator_back_trace_index: USHORT,
    object_type_index: UCHAR,
    /// `0x01` is `PROTECT_FROM_CLOSE`, `0x02` is `INHERIT`
    handle_attributes: UCHAR,
    handle: USHORT,
    object: PVOID,
    // granted_access: TOKEN_ACCESS_MASK,
    granted_access: ULONG,
}

#[repr(C)]
#[derive(Debug)]
struct SystemHandleInformation {
    number_of_handles: u32,
    handles: [SystemHandleTableEntryInfo; 1],
}

#[repr(C)]
struct ObjectTypeInformation {
    type_name: UNICODE_STRING,
    reserved: [ULONG; 22],
}

impl FdList {
    pub fn list(pid: Pid) -> Result<ProcStats, FshcError> {
        let mut stats = ProcStats::new(pid);

        let mut buffer: Vec<u8> = Vec::with_capacity(STEP);
        loop {
            buffer.resize(buffer.len() + STEP, 0u8);
            let mut return_length: ULONG = 0;
            match unsafe {
                nt_query_system_information(
                    SYSTEM_HANDLE_INFORMATION,
                    buffer.as_mut_ptr() as PVOID,
                    buffer.len() as u32,
                    &mut return_length,
                )
            } {
                STATUS_INFO_LENGTH_MISMATCH => continue,
                STATUS_SUCCESS => break,
                other => unimplemented!("Unknown error code: {other}"),
            }
        }

        let system_handle_info = unsafe { &*(buffer.as_ptr() as *const SystemHandleInformation) };
        let count = system_handle_info.number_of_handles as usize;
        let handles = unsafe {
            std::slice::from_raw_parts(
                system_handle_info.handles.as_ptr(),
                system_handle_info.number_of_handles as usize,
            )
        };

        let pid_u16 = pid.get() as u16;
        let file: Vec<_> = String::from("File").encode_utf16().collect();
        let file_slice = file.as_slice();
        for (i, handle) in handles.iter().enumerate() {
            if handle.unique_process_id == pid_u16 {
                let b2 = [0; 0x1000].as_mut_ptr();
                let mut return_length: ULONG = 0;
                match unsafe {
                    nt_query_object(
                        handle.handle as HANDLE,
                        2,
                        b2 as PVOID,
                        0x1000,
                        &mut return_length,
                    )
                } {
                    STATUS_SUCCESS => {
                        let object_info = unsafe { &*(b2 as *const ObjectTypeInformation) };
                        let units = unsafe {
                            std::slice::from_raw_parts(
                                object_info.type_name.Buffer,
                                object_info.type_name.Length as usize,
                            )
                        };
                        if units.len() >= 4 && &units[0..4] == file_slice {
                            stats.file_descriptors += 1;
                        }
                        let type_name = String::from_utf16_lossy(units);
                        eprintln!("handle ({i}/{count}): {}", type_name);
                    }
                    other => {
                        eprintln!("failed to look up handle ({i}/{count}): {other}");
                    }
                }
            }
        }

        Ok(stats)
    }
}
