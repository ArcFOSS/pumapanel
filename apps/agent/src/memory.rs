//! Reading the host's available memory.
//!
//! A server can be created with `all_memory`, meaning its JVM heap is sized from
//! whatever the host has free at creation time. That source sits behind a trait
//! so the agent's tests never depend on the machine they run on.

/// Where the agent learns how much memory the host can spare.
pub trait HostMemory: Send + Sync {
    /// Memory available to new processes, in MiB, or `None` when unknown.
    fn available_mb(&self) -> Option<u32>;
}

/// Reads `MemAvailable` from `/proc/meminfo`, the kernel's estimate of how much
/// memory a new process can take without forcing the host to swap.
pub struct ProcMeminfo;

impl HostMemory for ProcMeminfo {
    fn available_mb(&self) -> Option<u32> {
        parse_available_mb(&std::fs::read_to_string("/proc/meminfo").ok()?)
    }
}

/// Extracts `MemAvailable` (reported in kB) from `/proc/meminfo`, as MiB.
fn parse_available_mb(contents: &str) -> Option<u32> {
    let kib: u64 = contents
        .lines()
        .find_map(|line| line.strip_prefix("MemAvailable:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;

    u32::try_from(kib / 1024).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_available_memory_in_mib() {
        let meminfo = "\
MemTotal:       16329180 kB
MemFree:         1048576 kB
MemAvailable:    8388608 kB
SwapTotal:             0 kB
";
        assert_eq!(parse_available_mb(meminfo), Some(8192));
    }

    #[test]
    fn an_unknown_host_yields_none() {
        // `MemFree` is not a substitute: only `MemAvailable` is read.
        assert_eq!(parse_available_mb("MemFree: 1048576 kB\n"), None);
        assert_eq!(parse_available_mb("MemAvailable: lots\n"), None);
        assert_eq!(parse_available_mb(""), None);
    }
}
