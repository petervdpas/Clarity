//! The load of the computer and of Clarity, for Clarity settings, System,
//! and the meters of the status bar. A thread reads it once a second and
//! sends it as `system-load`. It reads only what the meters show (all
//! cores together, memory, disk activity) unless the System page is open
//! and asks for the rest with `watch_system`: each core, Clarity's
//! processes and the network.
//!
//! Clarity's share counts the IDE and every process it started, with what
//! those started in turn: a build's compilers and the application it runs.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{DiskRefreshKind, Disks, Networks, Pid, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

/// What the computer is.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MachineFacts {
    /// The maker and model, like `Lenovo IdeaPad S340-15IWL`; empty where the firmware does not say.
    pub machine: String,
    /// The computer's name on the network.
    pub host: String,
    /// Who is signed in.
    pub user: String,
    /// As the processor names itself; empty where the system does not say.
    pub processor: String,
    pub cores: u32,
    #[ts(type = "number")]
    pub memory: u64,
    /// The system and its version, like `Debian GNU/Linux 13`.
    pub system: String,
}

/// One reading. Shares are percents; bytes per second are over the time
/// since the reading before.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct LoadReading {
    /// Whether this reading has the System page's part too: each core,
    /// Clarity's processes and the network; without it those are empty.
    pub full: bool,
    /// Each core's share in use.
    pub cores: Vec<f32>,
    /// The share of all cores in use.
    pub cpu: f32,
    /// Clarity's share of all cores.
    pub clarity_cpu: f32,
    /// How many processes Clarity is.
    pub clarity_processes: u32,
    #[ts(type = "number")]
    pub memory_total: u64,
    #[ts(type = "number")]
    pub memory_used: u64,
    #[ts(type = "number")]
    pub clarity_memory: u64,
    #[ts(type = "number")]
    pub swap_total: u64,
    #[ts(type = "number")]
    pub swap_used: u64,
    #[ts(type = "number")]
    pub received: u64,
    #[ts(type = "number")]
    pub sent: u64,
    #[ts(type = "number")]
    pub received_total: u64,
    #[ts(type = "number")]
    pub sent_total: u64,
    #[ts(type = "number")]
    pub read: u64,
    #[ts(type = "number")]
    pub written: u64,
    #[ts(type = "number")]
    pub read_total: u64,
    #[ts(type = "number")]
    pub written_total: u64,
}

/// `root` and every process below it, from each process's parent.
pub fn family(root: u32, parents: &[(u32, u32)]) -> Vec<u32> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(pid, parent) in parents {
        if pid != parent {
            children.entry(parent).or_default().push(pid);
        }
    }
    let mut found = BTreeSet::from([root]);
    let mut todo = vec![root];
    while let Some(pid) = todo.pop() {
        for &child in children.get(&pid).into_iter().flatten() {
            if found.insert(child) {
                todo.push(child);
            }
        }
    }
    found.into_iter().collect()
}

/// How many readings the family is kept before all processes are looked at again.
const FAMILY_EVERY: u32 = 5;

/// Reads the load, keeping what the next reading needs.
pub struct Sampler {
    system: System,
    /// Made when a full reading first needs them.
    networks: Option<Networks>,
    disks: Disks,
    root: u32,
    family: Vec<Pid>,
    family_age: u32,
    at: Instant,
}

impl Sampler {
    pub fn new() -> Sampler {
        let mut system = System::new();
        system.refresh_cpu_usage();
        Sampler {
            system,
            networks: None,
            disks: Disks::new_with_refreshed_list_specifics(DiskRefreshKind::nothing().with_io_usage()),
            root: std::process::id(),
            family: Vec::new(),
            family_age: FAMILY_EVERY,
            at: Instant::now(),
        }
    }

    /// Reads the load; `full` adds the System page's part.
    pub fn read(&mut self, full: bool) -> LoadReading {
        let seconds = self.at.elapsed().as_secs_f64().max(0.001);
        self.at = Instant::now();
        let per_second = |bytes: u64| (bytes as f64 / seconds) as u64;

        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh_specifics(true, DiskRefreshKind::nothing().with_io_usage());
        // A partition mounted more than once is counted once.
        let mut seen = BTreeSet::new();
        let (mut read, mut written, mut read_total, mut written_total) = (0, 0, 0, 0);
        for disk in self.disks.list() {
            if !seen.insert(disk.name().to_os_string()) {
                continue;
            }
            let usage = disk.usage();
            read += usage.read_bytes;
            written += usage.written_bytes;
            read_total += usage.total_read_bytes;
            written_total += usage.total_written_bytes;
        }
        let mut reading = LoadReading {
            full,
            cpu: self.system.global_cpu_usage(),
            memory_total: self.system.total_memory(),
            memory_used: self.system.used_memory(),
            swap_total: self.system.total_swap(),
            swap_used: self.system.used_swap(),
            read: per_second(read),
            written: per_second(written),
            read_total,
            written_total,
            ..LoadReading::default()
        };
        if !full {
            // Found again in full when the page opens.
            self.family_age = FAMILY_EVERY;
            self.networks = None;
            return reading;
        }
        reading.cores = self.system.cpus().iter().map(|c| c.cpu_usage()).collect();

        // The family is found again every few readings; in between only its processes are asked.
        self.family_age += 1;
        if self.family_age >= FAMILY_EVERY {
            self.system.refresh_processes(ProcessesToUpdate::All, true);
            let parents: Vec<(u32, u32)> = self
                .system
                .processes()
                .iter()
                .filter(|(_, p)| p.thread_kind().is_none())
                .filter_map(|(pid, p)| p.parent().map(|parent| (pid.as_u32(), parent.as_u32())))
                .collect();
            self.family = family(self.root, &parents).into_iter().map(Pid::from_u32).collect();
            self.family_age = 0;
        } else {
            self.system.refresh_processes(ProcessesToUpdate::Some(&self.family), true);
        }
        let cores = self.system.cpus().len().max(1) as f32;
        for pid in &self.family {
            if let Some(p) = self.system.process(*pid) {
                reading.clarity_cpu += p.cpu_usage() / cores;
                reading.clarity_memory += p.memory();
                reading.clarity_processes += 1;
            }
        }
        reading.clarity_cpu = reading.clarity_cpu.min(100.0);

        // The first full reading only starts the network's count.
        let networks = match &mut self.networks {
            Some(networks) => {
                networks.refresh(true);
                networks
            }
            None => self.networks.insert(Networks::new_with_refreshed_list()),
        };
        for (name, data) in networks.iter() {
            // The loopback is the computer talking to itself: `lo` on Linux,
            // `lo0` on macOS, `Loopback Pseudo-Interface 1` on Windows.
            if name == "lo" || name == "lo0" || name.starts_with("Loopback") {
                continue;
            }
            reading.received += per_second(data.received());
            reading.sent += per_second(data.transmitted());
            reading.received_total += data.total_received();
            reading.sent_total += data.total_transmitted();
        }
        reading
    }
}


/// A disk's space, for the System page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiskSpace {
    /// The device or the volume's name: `nvme0n1p2`, `Windows`.
    pub name: String,
    /// Where it is mounted: `/`, `C:\`.
    pub mount: String,
    #[ts(type = "number")]
    pub total: u64,
    #[ts(type = "number")]
    pub available: u64,
    pub removable: bool,
}

/// A disk as the system lists it, before the System page's choice.
pub struct ListedDisk {
    pub space: DiskSpace,
    pub file_system: String,
    pub read_only: bool,
}

/// File systems that hold no files of the person's: packages mounted as
/// images, the firmware's variables, the EFI partition.
const NOT_SHOWN: [&str; 4] = ["squashfs", "efivarfs", "overlay", "vfat"];

/// The disks the page shows: what can be written and holds files, each
/// device once (at its shortest mount point), in mount order. The same on
/// every system, with macOS's shape: its `/` is a sealed read-only volume
/// that is shown, and the volumes under `/System/Volumes` share its space
/// and are not.
pub fn shown_disks(listed: Vec<ListedDisk>) -> Vec<DiskSpace> {
    let mut shown: Vec<DiskSpace> = Vec::new();
    for disk in listed {
        if disk.space.mount.starts_with("/System/Volumes/") {
            continue;
        }
        let root = disk.space.mount == "/";
        let kept = (root || !disk.read_only) && disk.space.total > 0 && !NOT_SHOWN.contains(&disk.file_system.as_str());
        // A removable FAT stick is shown; the EFI partition, also FAT, is not.
        let kept = kept || (disk.space.removable && disk.space.total > 0);
        if !kept {
            continue;
        }
        match shown.iter_mut().find(|d| d.name == disk.space.name) {
            Some(same) if disk.space.mount.len() < same.mount.len() => *same = disk.space,
            Some(_) => {}
            None => shown.push(disk.space),
        }
    }
    shown.sort_by(|a, b| a.mount.cmp(&b.mount));
    shown
}

/// The space on each disk, for the System page, which asks while it is open.
#[tauri::command(async)]
pub fn disk_space() -> Vec<DiskSpace> {
    let disks = Disks::new_with_refreshed_list_specifics(DiskRefreshKind::nothing().with_storage());
    shown_disks(
        disks
            .list()
            .iter()
            .map(|d| ListedDisk {
                space: DiskSpace {
                    name: d.name().to_string_lossy().trim_start_matches("/dev/").to_string(),
                    mount: d.mount_point().display().to_string(),
                    total: d.total_space(),
                    available: d.available_space(),
                    removable: d.is_removable(),
                },
                file_system: d.file_system().to_string_lossy().to_lowercase(),
                read_only: d.is_read_only(),
            })
            .collect(),
    )
}

/// What the computer is, for the System page.
#[tauri::command(async)]
pub fn machine_facts() -> MachineFacts {
    let mut system = System::new();
    system.refresh_cpu_all();
    system.refresh_memory();
    MachineFacts {
        machine: machine_name_here(),
        host: System::host_name().unwrap_or_default(),
        user: std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default(),
        processor: system.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
        cores: system.cpus().len() as u32,
        memory: system.total_memory(),
        system: System::long_os_version().or_else(System::name).unwrap_or_default(),
    }
}

/// What firmware writes where it has nothing to say.
const PLACEHOLDERS: [&str; 8] =
    ["", "none", "default string", "to be filled by o.e.m.", "system product name", "system version", "system manufacturer", "not applicable"];

fn said(value: &str) -> Option<&str> {
    let value = value.trim();
    (!PLACEHOLDERS.contains(&value.to_lowercase().as_str())).then_some(value)
}

/// The machine's name from the firmware's maker, product name and
/// version. Some makers put the model in the version and a type code in
/// the name (Lenovo: `81N8`, `Lenovo IdeaPad S340-15IWL`); the version is
/// the name then, when it names the maker.
pub fn machine_name(maker: &str, product: &str, version: &str) -> String {
    let maker = said(maker).unwrap_or("");
    if let Some(version) = said(version) {
        let word = maker.split_whitespace().next().unwrap_or("").to_lowercase();
        if !word.is_empty() && version.to_lowercase().contains(&word) {
            return version.to_string();
        }
    }
    match said(product) {
        Some(product) if product.to_lowercase().starts_with(&maker.to_lowercase()) => product.to_string(),
        Some(product) => format!("{maker} {product}").trim().to_string(),
        None => maker.to_string(),
    }
}

/// The value of `name` in what `reg query` prints:
/// `    SystemManufacturer    REG_SZ    LENOVO`.
pub fn registry_value(text: &str, name: &str) -> String {
    text.lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next() == Some(name) && parts.next() == Some("REG_SZ")).then(|| parts.collect::<Vec<_>>().join(" "))
        })
        .unwrap_or_default()
}

/// The machine's name on this system.
fn machine_name_here() -> String {
    #[cfg(target_os = "linux")]
    {
        let read = |field: &str| std::fs::read_to_string(format!("/sys/class/dmi/id/{field}")).unwrap_or_default();
        machine_name(&read("sys_vendor"), &read("product_name"), &read("product_version"))
    }
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("reg")
            .args(["query", r"HKLM\HARDWARE\DESCRIPTION\System\BIOS"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        machine_name(
            &registry_value(&output, "SystemManufacturer"),
            &registry_value(&output, "SystemProductName"),
            &registry_value(&output, "SystemVersion"),
        )
    }
    #[cfg(target_os = "macos")]
    {
        let model = std::process::Command::new("sysctl")
            .args(["-n", "hw.model"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        machine_name("Apple", &model, "")
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        String::new()
    }
}

/// Whether the System page is open, so the readings are full.
static WATCHED: AtomicBool = AtomicBool::new(false);

/// The System page says it opened or closed.
#[tauri::command]
pub fn watch_system(watched: bool) {
    WATCHED.store(watched, Ordering::Relaxed);
}

/// Reads the load once a second for as long as Clarity runs.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut sampler = Sampler::new();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let _ = app.emit("system-load", sampler.read(WATCHED.load(Ordering::Relaxed)));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_family_is_the_root_and_everything_below_it() {
        // 10 is Clarity; 11 a build it started, 12 the compiler the build
        // started; 20 is somebody else's, with a child of its own.
        let parents = [(11, 10), (12, 11), (20, 1), (21, 20), (10, 1)];
        assert_eq!(family(10, &parents), [10, 11, 12]);
        assert_eq!(family(99, &parents), [99]);
        // A loop in the parents does not walk forever.
        assert_eq!(family(10, &[(11, 10), (10, 11)]), [10, 11]);
    }

    #[test]
    fn the_machine_is_named_by_its_maker_and_model() {
        assert_eq!(machine_name("LENOVO\n", "81N8\n", "Lenovo IdeaPad S340-15IWL\n"), "Lenovo IdeaPad S340-15IWL");
        assert_eq!(machine_name("Dell Inc.", "XPS 13 9380", ""), "Dell Inc. XPS 13 9380");
        assert_eq!(machine_name("HP", "HP EliteBook 840 G8", "SBKPF"), "HP EliteBook 840 G8");
        assert_eq!(machine_name("To Be Filled By O.E.M.", "To Be Filled By O.E.M.", "Default string"), "");
        assert_eq!(machine_name("Apple", "MacBookPro18,3", ""), "Apple MacBookPro18,3");
        let printed = "\r\nHKEY_LOCAL_MACHINE\\HARDWARE\\DESCRIPTION\\System\\BIOS\r\n    SystemManufacturer    REG_SZ    LENOVO\r\n    SystemVersion    REG_SZ    Lenovo IdeaPad S340-15IWL\r\n";
        assert_eq!(registry_value(printed, "SystemVersion"), "Lenovo IdeaPad S340-15IWL");
        assert_eq!(registry_value(printed, "SystemProductName"), "");
    }

    fn listed(name: &str, mount: &str, file_system: &str, read_only: bool, removable: bool) -> ListedDisk {
        ListedDisk {
            space: DiskSpace { name: name.into(), mount: mount.into(), total: 100, available: 40, removable },
            file_system: file_system.into(),
            read_only,
        }
    }

    #[test]
    fn the_page_shows_each_disk_that_holds_files_once() {
        let disks = vec![
            listed("nvme0n1p2", "/home", "btrfs", false, false),
            listed("nvme0n1p2", "/", "btrfs", false, false),
            listed("nvme0n1p1", "/boot/efi", "vfat", false, false),
            listed("loop3", "/snap/gnome/198", "squashfs", true, false),
            listed("sda1", "/media/stick", "vfat", false, true),
        ];
        let names: Vec<(String, String)> = shown_disks(disks).into_iter().map(|d| (d.name, d.mount)).collect();
        assert_eq!(names, [("nvme0n1p2".to_string(), "/".to_string()), ("sda1".to_string(), "/media/stick".to_string())]);
        // macOS: a sealed `/` and its data volume.
        let mac = vec![listed("disk3s1s1", "/", "apfs", true, false), listed("disk3s5", "/System/Volumes/Data", "apfs", false, false)];
        assert_eq!(shown_disks(mac).into_iter().map(|d| d.mount).collect::<Vec<_>>(), ["/"]);
        // Windows: drives by letter.
        let windows = vec![listed("Windows", "C:\\", "ntfs", false, false), listed("Data", "D:\\", "ntfs", false, false)];
        assert_eq!(shown_disks(windows).len(), 2);
    }

    #[test]
    fn a_reading_counts_clarity_among_the_processes() {
        let mut sampler = Sampler::new();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        // The status bar's reading leaves the page's part out.
        let light = sampler.read(false);
        assert!(!light.full && light.cores.is_empty() && light.clarity_processes == 0);
        assert!(light.memory_total > 0);
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        let reading = sampler.read(true);
        assert!(reading.full && !reading.cores.is_empty());
        assert!(reading.memory_total > 0 && reading.memory_used <= reading.memory_total);
        assert!(reading.clarity_processes >= 1);
        assert!(reading.clarity_memory > 0);
        assert!(machine_facts().cores >= 1);
    }
}
