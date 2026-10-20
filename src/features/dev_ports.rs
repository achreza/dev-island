use std::path::Path;
use std::time::Instant;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivePort {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
}

pub struct DevPortsMonitor {
    pub interesting_ports: Vec<u16>,
    pub cached_active: Vec<ActivePort>,
    pub last_checked: Option<Instant>,
    pub cache_ttl_secs: f32,
}

impl DevPortsMonitor {
    pub fn new() -> Self {
        Self {
            // Common web/backend developer ports
            interesting_ports: vec![3000, 3001, 4200, 5173, 8000, 8080, 8081],
            cached_active: Vec::new(),
            last_checked: None,
            cache_ttl_secs: 3.0,
        }
    }

    pub fn get_active_ports(&mut self) -> &[ActivePort] {
        if let Some(last) = self.last_checked {
            if last.elapsed().as_secs_f32() < self.cache_ttl_secs {
                return &self.cached_active;
            }
        }

        self.cached_active = self.query_active_ports();
        self.last_checked = Some(Instant::now());
        &self.cached_active
    }

    fn query_active_ports(&self) -> Vec<ActivePort> {
        let mut results = Vec::new();

        unsafe {
            let mut size: u32 = 0;
            // First call gets the required buffer size
            // AF_INET = 2
            let _ = GetExtendedTcpTable(None, &mut size, false, 2, TCP_TABLE_OWNER_PID_ALL, 0);

            if size == 0 {
                return results;
            }

            let mut buffer: Vec<u8> = vec![0; size as usize];
            if GetExtendedTcpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut size,
                false,
                2,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ) != 0 {
                return results;
            }

            let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
            let num_entries = table.dwNumEntries as usize;
            let rows_ptr = table.table.as_ptr();

            for i in 0..num_entries {
                let row = &*rows_ptr.add(i);
                // dwState == 2 is MIB_TCP_STATE_LISTEN
                if row.dwState == 2 {
                    // Local port is in big-endian in the low 16 bits
                    let port = u16::from_be((row.dwLocalPort & 0xFFFF) as u16);
                    if self.interesting_ports.contains(&port) || (port >= 3000 && port <= 3010) || port == 5173 || (port >= 8000 && port <= 8085) {
                        let pid = row.dwOwningPid;
                        let proc_name = Self::query_process_name(pid).unwrap_or_else(|| format!("PID {}", pid));
                        results.push(ActivePort {
                            port,
                            pid,
                            process_name: proc_name,
                        });
                    }
                }
            }
        }

        // De-duplicate by port
        results.sort_by_key(|p| p.port);
        results.dedup_by_key(|p| p.port);
        results
    }

    fn query_process_name(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buffer = [0u16; 1024];
            let mut len = buffer.len() as u32;

            let success = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buffer.as_mut_ptr()),
                &mut len,
            ).is_ok();

            let _ = CloseHandle(handle);

            if success && len > 0 {
                let full_path = String::from_utf16_lossy(&buffer[..len as usize]);
                let file_name = Path::new(&full_path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or(full_path);
                Some(file_name)
            } else {
                None
            }
        }
    }

    pub fn kill_process(pid: u32) -> bool {
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                let success = TerminateProcess(handle, 1).is_ok();
                let _ = CloseHandle(handle);
                if success {
                    println!("Terminated stuck dev server PID {}", pid);
                }
                success
            } else {
                eprintln!("Failed to open process PID {} for termination", pid);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interesting_ports_contain_defaults() {
        let monitor = DevPortsMonitor::new();
        assert!(monitor.interesting_ports.contains(&3000));
        assert!(monitor.interesting_ports.contains(&5173));
        assert!(monitor.interesting_ports.contains(&8080));
    }
}
