use std::sync::OnceLock;

#[cfg(windows)]
pub struct ProcessJobGroup {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for ProcessJobGroup {}
#[cfg(windows)]
unsafe impl Sync for ProcessJobGroup {}

#[cfg(windows)]
impl ProcessJobGroup {
    pub fn new() -> Option<Self> {
        use windows_sys::Win32::Foundation::*;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return None;
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let res = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if res == 0 {
                CloseHandle(handle);
                return None;
            }

            Some(Self { handle })
        }
    }

    pub fn assign_child(&self, child: &std::process::Child) -> bool {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        let raw_handle = child.as_raw_handle();
        unsafe {
            AssignProcessToJobObject(self.handle, raw_handle as _) != 0
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessJobGroup {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::*;
        unsafe {
            if !self.handle.is_null() && self.handle != INVALID_HANDLE_VALUE {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(not(windows))]
pub struct ProcessJobGroup;

#[cfg(not(windows))]
impl ProcessJobGroup {
    pub fn new() -> Option<Self> {
        Some(Self)
    }

    pub fn assign_child(&self, _child: &std::process::Child) -> bool {
        true
    }
}

static GLOBAL_JOB_GROUP: OnceLock<Option<ProcessJobGroup>> = OnceLock::new();

pub fn get_global_job_group() -> Option<&'static ProcessJobGroup> {
    GLOBAL_JOB_GROUP.get_or_init(ProcessJobGroup::new).as_ref()
}

pub fn assign_child_to_global_job(child: &std::process::Child) {
    if let Some(job) = get_global_job_group() {
        job.assign_child(child);
    }
}
