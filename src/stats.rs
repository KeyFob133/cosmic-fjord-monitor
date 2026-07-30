//! Sampling of the numbers the widget displays.
//!
//! CPU and memory come from `sysinfo`. Temperatures and GPU load are read
//! straight out of sysfs where possible: the kernel interfaces are stable, they
//! cost one `read_to_string` each, and it avoids depending on a vendor tool.
//!
//! GPU utilisation is the awkward one. amdgpu and recent i915/xe expose
//! `gpu_busy_percent`; older i915 and NVIDIA's proprietary driver expose nothing
//! equivalent. On NVIDIA we fall back to `nvidia-smi`, which costs a process
//! spawn of a few hundred milliseconds — far too slow to call from the render
//! loop, so it runs on its own thread and the sampler reads whatever it last
//! published.
//!
//! Anything we cannot determine is reported as `None` rather than zero, so the UI
//! can distinguish "idle" from "unknown".

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// One sample. Loads are fractions in `0.0..=1.0`, not percentages.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub cpu_load: f32,
    pub cpu_temp_c: Option<f32>,
    /// Logical threads currently working, and how many exist in total.
    pub busy_threads: u16,
    pub total_threads: u16,
    pub mem_used_gib: f32,
    pub mem_total_gib: f32,
    pub gpu_load: Option<f32>,
    pub gpu_temp_c: Option<f32>,
}

impl Sample {
    pub fn mem_load(&self) -> f32 {
        if self.mem_total_gib <= 0.0 {
            0.0
        } else {
            (self.mem_used_gib / self.mem_total_gib).clamp(0.0, 1.0)
        }
    }
}

/// Latest utilisation and temperature published by the polling thread.
type NvidiaReading = Arc<Mutex<Option<(f32, f32)>>>;

/// Where GPU numbers come from on this machine, decided once at startup.
enum GpuSource {
    /// Kernel counter. Cheap, read synchronously.
    Sysfs {
        busy: PathBuf,
        temp: Option<PathBuf>,
    },
    /// `nvidia-smi`, polled on a background thread.
    Nvidia(NvidiaReading),
}

pub struct Monitor {
    system: System,
    cpu_temp: Option<PathBuf>,
    gpu: Option<GpuSource>,
}

impl Monitor {
    /// `gpu_poll` is how often the `nvidia-smi` thread runs. It is ignored when a
    /// sysfs counter is available.
    pub fn new(gpu_poll: Duration) -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );

        // Prefer the kernel counter on any card that has one. On a hybrid laptop
        // sysfs may turn up nothing at all — Raptor Lake graphics do not publish
        // a utilisation counter — so fall back to NVIDIA before giving up.
        let gpu = find_sysfs_gpu()
            .map(|g| GpuSource::Sysfs {
                busy: g.busy,
                temp: g.temp,
            })
            .or_else(|| spawn_nvidia_poller(gpu_poll).map(GpuSource::Nvidia));

        Self {
            system,
            cpu_temp: find_cpu_temp(),
            gpu,
        }
    }

    /// Take a reading. Call this no more than once per
    /// `sysinfo::MINIMUM_CPU_UPDATE_INTERVAL` or CPU load will read as zero.
    pub fn sample(&mut self) -> Sample {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        const GIB: f32 = 1024.0 * 1024.0 * 1024.0;

        // A thread above this fraction of its own capacity counts as working.
        // Half is a deliberate midpoint: below it a thread is doing less than it
        // is idling, and scheduler noise on a quiet machine rarely sustains it.
        const BUSY: f32 = 0.5;

        let cpus = self.system.cpus();
        let total_threads = cpus.len() as u16;
        let busy_threads = cpus
            .iter()
            .filter(|cpu| cpu.cpu_usage() / 100.0 >= BUSY)
            .count() as u16;

        let (gpu_load, gpu_temp_c) = self.read_gpu();

        Sample {
            cpu_load: (self.system.global_cpu_usage() / 100.0).clamp(0.0, 1.0),
            cpu_temp_c: self.cpu_temp.as_deref().and_then(read_millidegrees),
            busy_threads,
            total_threads,
            mem_used_gib: self.system.used_memory() as f32 / GIB,
            mem_total_gib: self.system.total_memory() as f32 / GIB,
            gpu_load,
            gpu_temp_c,
        }
    }

    fn read_gpu(&self) -> (Option<f32>, Option<f32>) {
        match &self.gpu {
            Some(GpuSource::Sysfs { busy, temp }) => (
                read_percent(busy).map(|p| p.clamp(0.0, 1.0)),
                temp.as_deref().and_then(read_millidegrees),
            ),
            Some(GpuSource::Nvidia(shared)) => {
                // A poisoned mutex means the polling thread panicked while
                // holding it. Report "unknown" rather than bringing the widget
                // down along with it.
                match shared.lock() {
                    Ok(slot) => match *slot {
                        Some((util, temp)) => (Some((util / 100.0).clamp(0.0, 1.0)), Some(temp)),
                        None => (None, None),
                    },
                    Err(_) => (None, None),
                }
            }
            None => (None, None),
        }
    }
}

/// hwmon reports temperatures in thousandths of a degree Celsius.
fn read_millidegrees(path: &Path) -> Option<f32> {
    let raw: f32 = fs::read_to_string(path).ok()?.trim().parse().ok()?;
    let celsius = raw / 1000.0;
    // Guard against a sensor that has gone away and now reports nonsense.
    (celsius > -50.0 && celsius < 150.0).then_some(celsius)
}

fn read_percent(path: &Path) -> Option<f32> {
    let raw: f32 = fs::read_to_string(path).ok()?.trim().parse().ok()?;
    Some(raw / 100.0)
}

/// Walk `/sys/class/hwmon` for the package temperature of the CPU.
///
/// Driver names differ by vendor: `k10temp` and `zenpower` on AMD, `coretemp` on
/// Intel. Within a driver we prefer a sensor labelled as the package or Tctl,
/// falling back to `temp1_input`, which is the package on every driver above.
fn find_cpu_temp() -> Option<PathBuf> {
    const DRIVERS: [&str; 3] = ["k10temp", "zenpower", "coretemp"];
    const LABELS: [&str; 3] = ["Tctl", "Package id 0", "Tdie"];

    for driver in DRIVERS {
        let Some(dir) = hwmon_dir_named(driver) else {
            continue;
        };

        for index in 1..=8 {
            let label_path = dir.join(format!("temp{index}_label"));
            let Ok(label) = fs::read_to_string(&label_path) else {
                continue;
            };
            if LABELS.iter().any(|wanted| label.trim() == *wanted) {
                let input = dir.join(format!("temp{index}_input"));
                if input.exists() {
                    return Some(input);
                }
            }
        }

        let fallback = dir.join("temp1_input");
        if fallback.exists() {
            return Some(fallback);
        }
    }

    None
}

struct SysfsGpu {
    busy: PathBuf,
    temp: Option<PathBuf>,
}

/// Find a DRM card that exposes a utilisation counter.
fn find_sysfs_gpu() -> Option<SysfsGpu> {
    let entries = fs::read_dir("/sys/class/drm").ok()?;

    let mut cards: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("card") && !n.contains('-'))
        })
        .collect();
    cards.sort();

    for card in cards {
        let device = card.join("device");
        let busy = device.join("gpu_busy_percent");
        if !busy.exists() {
            continue;
        }

        // The card's own hwmon node carries edge/junction temperature.
        let temp = fs::read_dir(device.join("hwmon"))
            .ok()
            .and_then(|mut d| d.next())
            .and_then(|e| e.ok())
            .map(|e| e.path().join("temp1_input"))
            .filter(|p| p.exists());

        return Some(SysfsGpu { busy, temp });
    }

    None
}

/// Probe `nvidia-smi` once and, if it answers, keep a thread polling it.
///
/// The probe is synchronous so a machine without the tool decides instantly and
/// never spawns a thread. After that the widget only ever reads a cached value,
/// so a slow or hanging `nvidia-smi` cannot stall the render loop.
///
/// Note the cost on hybrid graphics: querying the discrete card can keep it
/// powered up. If that matters on battery, raise `gpu_poll_interval`.
fn spawn_nvidia_poller(interval: Duration) -> Option<NvidiaReading> {
    let first = query_nvidia()?;

    let shared: NvidiaReading = Arc::new(Mutex::new(Some(first)));
    let writer = Arc::clone(&shared);

    let spawned = std::thread::Builder::new()
        .name("nvidia-poll".to_string())
        .spawn(move || loop {
            std::thread::sleep(interval);
            let reading = query_nvidia();
            match writer.lock() {
                Ok(mut slot) => *slot = reading,
                // Lock poisoned: stop polling rather than spinning forever.
                Err(_) => break,
            }
        });

    match spawned {
        Ok(_) => Some(shared),
        // Thread creation failed. Return the single reading already taken rather
        // than nothing: a stale number beats a blank gauge.
        Err(error) => {
            eprintln!("fjord-monitor: nvidia poll thread: {error}");
            Some(shared)
        }
    }
}

/// One `nvidia-smi` query, as `(utilisation percent, degrees Celsius)`.
fn query_nvidia() -> Option<(f32, f32)> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // First line only: a multi-GPU machine prints one row per card.
    let text = String::from_utf8_lossy(&output.stdout);
    let mut fields = text.lines().next()?.split(',');

    let util = leading_number(fields.next()?)?;
    let temp = leading_number(fields.next()?)?;
    Some((util, temp))
}

/// Parse the number at the start of a field, ignoring any trailing unit.
///
/// `nounits` should mean there is nothing to ignore, but the flag has not been
/// honoured for every field across driver versions, so this accepts both `37`
/// and `37 %`.
fn leading_number(field: &str) -> Option<f32> {
    let digits: String = field
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    digits.parse().ok()
}

fn hwmon_dir_named(name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let dir = entry.path();
        if let Ok(found) = fs::read_to_string(dir.join("name")) {
            if found.trim() == name {
                return Some(dir);
            }
        }
    }
    None
}
