// run  := cargo run --
// dir  := .
// kid  :=

extern crate libc;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::fs::File;
use std::io::BufRead;
use std::os::raw::c_char;
use std::path::Path;
use std::process::Command;
use std::{fs, io, mem, str};

use libc::{statvfs, statvfs as statvfs_t};
use regex::Regex;
use serde_json::Value;

use crate::load_config::{Config, SysDisk, SysDocker, SysGpu, SysService, SysVm};
use crate::utils::str2byte;

#[derive(Debug)]
pub struct Service {
    pub name:     String,
    pub memory:   u64,
    pub state:    String,
    pub substate: String
}

#[derive(Debug)]
pub struct Docker {
    pub name:   String,
    pub state:  String,
    pub status: String
}

#[derive(Debug)]
pub struct MemInfo {
    pub total_mem:     u64,
    pub free_mem:      u64,
    pub active_mem:    u64,
    pub buffer:        u64,
    pub cache:         u64,
    pub available_mem: u64,
    pub total_swap:    u64,
    pub free_swap:     u64
}

#[derive(Debug)]
pub struct LoadAvgInfo {
    pub one:     f64,
    pub five:    f64,
    pub fifteen: f64
}

#[derive(Debug)]
pub struct LoginInfo {
    pub user: String,
    pub host: String
}

#[derive(Debug)]
pub struct DiskInfo {
    pub name:   String,
    pub total:  u64,
    pub used:   u64,
    pub subvol: Vec<(String, u64)>
}

#[derive(Debug)]
pub struct VmInfo {
    pub name:      String,
    pub state:     String,
    pub cpus:      u64,
    pub used_mem:  u64,
    pub total_mem: u64,
    pub autostart: String
}

#[derive(Debug)]
pub struct GpuInfo {
    pub mem_name:   String,
    pub temp_name:  String,
    pub temp:       u64,
    pub used_vram:  u64,
    pub total_vram: u64
}

#[derive(Debug)]
pub struct SystemStats {
    pub memory:     MemInfo,
    pub load_avg:   LoadAvgInfo,
    pub cpu_temp:   Vec<(String, u64)>,
    pub uptime:     u64,
    pub last_login: LoginInfo,
    pub disks:      Vec<DiskInfo>,
    pub services:   Vec<Service>,
    pub dockers:    Vec<Docker>,
    pub vms:        Vec<VmInfo>,
    pub gpus:       Vec<GpuInfo>
}

pub fn get_memory() -> io::Result<MemInfo> {
    let path = Path::new("/proc/meminfo");
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut mem_info = MemInfo {
        total_mem:     0,
        free_mem:      0,
        active_mem:    0,
        buffer:        0,
        cache:         0,
        available_mem: 0,
        total_swap:    0,
        free_swap:     0
    };

    let keys_of_interest: HashSet<&str> =
        ["MemTotal:", "MemFree:", "MemAvailable:", "Buffers:", "Active:", "Cached:", "SwapTotal:", "SwapFree:"]
            .iter()
            .cloned()
            .collect();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if let Some(key) = parts.first() {
            if keys_of_interest.contains(key) {
                if let Some(value_str) = parts.get(1) {
                    let value = value_str.parse::<u64>().unwrap() * 1024;
                    match *key {
                        "MemTotal:" => mem_info.total_mem = value,
                        "MemFree:" => mem_info.free_mem = value,
                        "MemAvailable:" => mem_info.available_mem = value,
                        "Buffers:" => mem_info.buffer = value,
                        "Active:" => mem_info.active_mem = value,
                        "Cached:" => mem_info.cache = value,
                        "SwapTotal:" => mem_info.total_swap = value,
                        "SwapFree:" => mem_info.free_swap = value,
                        _ => ()
                    }
                }
            }
        }
    }

    Ok(mem_info)
}

pub fn get_load() -> LoadAvgInfo {
    let mut loadavg = [0.0_f64; 3];
    let result = unsafe { libc::getloadavg(loadavg.as_mut_ptr(), 3) };

    if result != -1 {
        LoadAvgInfo { one: loadavg[0], five: loadavg[1], fifteen: loadavg[2] }
    }
    else {
        eprintln!("Failed to get load average");
        std::process::exit(1);
    }
}

fn get_cpu_temp(cpu_restr: &str) -> io::Result<Vec<(String, u64)>> {
    let mut temperatures = Vec::new();
    let hwmon_paths = fs::read_dir("/sys/class/hwmon/")?;

    let regex = Regex::new(cpu_restr).expect("Invalid regular expression");

    for hwmon_path in hwmon_paths {
        let hwmon_path = hwmon_path?.path();
        if let Ok(name) = fs::read_to_string(hwmon_path.join("name")) {
            if regex.is_match(name.trim()) {
                for entry in fs::read_dir(&hwmon_path)? {
                    let entry = entry?;
                    let filename = entry.file_name().into_string().unwrap();
                    if filename.starts_with("temp") && filename.ends_with("_input") {
                        let temp_file = hwmon_path.join(&filename);
                        let temp = fs::read_to_string(&temp_file)?.trim().parse::<u64>().unwrap();

                        let label_filename = filename.replace("_input", "_label");
                        let label_file = hwmon_path.join(&label_filename);
                        let label = if label_file.exists() {
                            fs::read_to_string(&label_file)?.trim().to_string()
                        }
                        else {
                            "Unknown".to_string()
                        };

                        temperatures.push((label, temp));
                    }
                }
            }
        }
    }
    Ok(temperatures)
}

fn get_uptime() -> u64 {
    unsafe {
        let mut info: libc::sysinfo = mem::zeroed();
        if libc::sysinfo(&mut info) == 0 {
            info.uptime as u64
        }
        else {
            0
        }
    }
}

pub fn get_last_login() -> LoginInfo {
    let output = Command::new("last")
        .arg("-T")
        .arg("-w")
        .arg("-i")
        .arg("-n")
        .arg("2")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let output_str = str::from_utf8(&output.stdout).unwrap_or("Invalid UTF-8");
        let lines: Vec<&str> = output_str.lines().collect();
        if lines.len() > 1 {
            let last_login_line = lines[1];
            let parts: Vec<&str> = last_login_line.split('\t').collect();
            if parts.len() >= 3 {
                let user = parts[0].trim().to_string();
                let host = parts[2].trim().to_string();
                return LoginInfo { user, host };
            }
        }
    }
    else {
        let error_str = str::from_utf8(&output.stderr).unwrap_or("Invalid UTF-8");
        eprintln!("Error: {}", error_str);
    }
    LoginInfo { user: "".to_string(), host: "".to_string() }
}

pub fn get_service(service: &SysService) -> Service {
    let service_name = if service.display != "none" { &service.display } else { &service.name };
    let output = Command::new("systemctl")
        .arg("show")
        .arg(&service.name)
        .arg("--property=ActiveState,SubState,MemoryCurrent")
        .output()
        .expect("Failed to execute command");
    if output.status.success() {
        let output_str = str::from_utf8(&output.stdout).unwrap_or("Invalid UTF-8");
        let mut memory_current: u64 = 0;
        let mut active_state = String::new();
        let mut sub_state = String::new();
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                match parts[0] {
                    "MemoryCurrent" => memory_current = parts[1].parse().unwrap_or(0),
                    "ActiveState" => active_state = parts[1].to_string(),
                    "SubState" => sub_state = parts[1].to_string(),
                    _ => {}
                }
            }
        }
        return Service {
            name:     service_name.to_string(),
            memory:   memory_current,
            state:    active_state,
            substate: sub_state
        };
    }
    else {
        let error_str = str::from_utf8(&output.stderr).unwrap_or("Invalid UTF-8");
        eprintln!("Error: {}", error_str);
    }
    Service {
        name:     service_name.to_string(),
        memory:   0,
        state:    "unknown".to_string(),
        substate: "unknown".to_string()
    }
}

pub fn get_service_all(systemctl: &Vec<SysService>) -> Vec<Service> {
    let mut services: Vec<Service> = Vec::new();
    for service in systemctl {
        let service_info = get_service(service);
        services.push(service_info);
    }
    services
}

pub fn get_docker(containers: &Vec<SysDocker>) -> Vec<Docker> {
    let mut container_map: HashMap<String, (String, String)> = HashMap::new();
    let output = Command::new("docker")
        .arg("ps")
        .arg("--format")
        .arg("{{.Names}}:{{.State}}:{{.Status}}")
        .output()
        .expect("Failed to execute command");
    if output.status.success() {
        let output_str = str::from_utf8(&output.stdout).unwrap_or("Invalid UTF-8");
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 3 {
                let name = parts[0].to_string();
                let state = parts[1].to_string();
                let status = parts[2].to_string();
                container_map.insert(name, (state, status));
            }
        }
    }
    else {
        let error_str = str::from_utf8(&output.stderr).unwrap_or("Invalid UTF-8");
        eprintln!("Error: {}", error_str);
    }

    let mut dockers: Vec<Docker> = Vec::new();
    for container in containers {
        if let Some((state, status)) = container_map.get(&container.name) {
            dockers.push(Docker {
                name:   if container.display != "none" {
                    container.display.to_string()
                }
                else {
                    container.name.to_string()
                },
                state:  state.to_string(),
                status: status.to_string()
            });
        }
    }
    dockers
}

fn get_disk_usage(disk_config: &SysDisk) -> Result<DiskInfo, String> {
    let path_c = CString::new(disk_config.path.as_str()).map_err(|e| e.to_string())?;
    let mut stat: statvfs_t = unsafe { mem::zeroed() };

    let ret = unsafe { statvfs(path_c.as_ptr() as *const c_char, &mut stat) };
    if ret != 0 {
        return Err("Failed to get disk usage".to_string());
    }

    let name = if disk_config.display != "none" { &disk_config.display } else { &disk_config.path };
    let total = stat.f_blocks * stat.f_frsize as u64;
    let used = (stat.f_blocks - stat.f_bfree) * stat.f_frsize as u64;
    let mut subvol: Vec<(String, u64)> = Vec::new();
    if !disk_config.subvol.is_empty() {
        let output = Command::new("btrfs")
            .arg("--format")
            .arg("json")
            .arg("qgroup")
            .arg("show")
            .arg(disk_config.path.as_str())
            .output()
            .expect("Failed to execute command");
        if output.status.success() {
            let output_str = str::from_utf8(&output.stdout).unwrap_or("Invalid UTF-8");

            let parsed_json: Value = serde_json::from_str(output_str).expect("Failed to parse JSON");
            let mut path_reference_map: HashMap<String, u64> = HashMap::new();
            if let Some(qgroup_show) = parsed_json.get("qgroup-show").and_then(|v| v.as_array()) {
                for entry in qgroup_show {
                    if let (Some(path), Some(referenced)) =
                        (entry.get("path").and_then(|v| v.as_str()), entry.get("referenced").and_then(|v| v.as_u64()))
                    {
                        path_reference_map.insert(path.to_string(), referenced);
                    }
                }
            }
            for subvol_name in &disk_config.subvol {
                if let Some(referenced) = path_reference_map.get(subvol_name) {
                    subvol.push((subvol_name.to_string(), *referenced));
                }
            }
        }
        else {
            let error_str = str::from_utf8(&output.stderr).unwrap_or("Invalid UTF-8");
            eprintln!("Error: {}", error_str);
        }
    }
    Ok(DiskInfo { name: name.to_string(), total, used, subvol })
}

fn get_disks(disks_config: &Vec<SysDisk>) -> Vec<DiskInfo> {
    let mut disks: Vec<DiskInfo> = Vec::new();
    for sysdisk in disks_config {
        if let Ok(disk_info) = get_disk_usage(sysdisk) {
            disks.push(disk_info);
        }
    }
    disks
}

fn get_vm(vm_config: &SysVm) -> io::Result<VmInfo> {
    let output = Command::new("virsh").arg("dominfo").arg(vm_config.name.clone()).output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, String::from_utf8_lossy(&output.stderr).to_string()));
    }
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut info_map = HashMap::new();
    for line in output_str.lines() {
        if let Some((key, value)) = line.split_once(":") {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            match key.as_str() {
                "state" | "cpu(s)" | "used memory" | "max memory" | "autostart" => {
                    info_map.insert(key, value);
                }
                _ => {}
            }
        }
    }
    let name = (if vm_config.display != "none" { &vm_config.display } else { &vm_config.name }).clone();
    let state = info_map.get("state").unwrap_or(&"Unknown".to_string()).clone();
    let cpus = info_map.get("cpu(s)").unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0);
    let used_mem = str2byte(info_map.get("used memory").unwrap_or(&"0".to_string()));
    let total_mem = str2byte(info_map.get("max memory").unwrap_or(&"0".to_string()));
    let autostart = info_map.get("autostart").unwrap_or(&"Unknown".to_string()).clone();
    Ok(VmInfo { name, state, cpus, used_mem, total_mem, autostart })
}

fn get_vms(vms_config: &Vec<SysVm>) -> Vec<VmInfo> {
    let mut vms: Vec<VmInfo> = Vec::new();
    for vm in vms_config {
        if let Ok(vm_info) = get_vm(vm) {
            vms.push(vm_info);
        }
    }
    vms
}

fn get_nvidia_smi(gpu_config: &SysGpu) -> io::Result<GpuInfo> {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=memory.used,memory.total,temperature.gpu")
        .arg("--format=csv,noheader,nounits")
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, String::from_utf8_lossy(&output.stderr).to_string()));
    }
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut gpu_info = GpuInfo { mem_name: "VRAM".to_string(), temp_name: "GPU Core".to_string(), temp: 0, used_vram: 0, total_vram: 0 };
    let lines: Vec<&str> = output_str.lines().collect();
    if let Some(line) = lines.get(gpu_config.command.parse::<usize>().unwrap_or(0)) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 3 {
            gpu_info.mem_name = if gpu_config.memdisplay != "none" { &gpu_config.memdisplay } else { "VRAM" }.to_string();
            gpu_info.temp_name = if gpu_config.tempdisplay != "none" { &gpu_config.tempdisplay } else { "GPU Core" }.to_string();
            gpu_info.used_vram = parts[0].trim().parse::<u64>().unwrap_or(0) * 1024 * 1024;
            gpu_info.total_vram = parts[1].trim().parse::<u64>().unwrap_or(0) * 1024 * 1024;
            gpu_info.temp = parts[2].trim().parse::<u64>().unwrap_or(0);
        }
    }
    Ok(gpu_info)
}

fn get_gpus(gpus_config: &Vec<SysGpu>) -> Vec<GpuInfo> {
    let mut gpus: Vec<GpuInfo> = Vec::new();
    for gpu in gpus_config {
        if gpu.command == "nvidia-smi" {
            if let Ok(gpu_info) = get_nvidia_smi(gpu) {
                gpus.push(gpu_info);
            }
        }
    }
    gpus
}

impl SystemStats {
    pub fn new(config: &Config) -> Self {
        let memory = get_memory().unwrap_or_else(|e| {
            eprintln!("Failed to get memory information: {}", e);
            std::process::exit(1);
        });
        let load_avg = get_load();
        let uptime = get_uptime();
        let last_login = get_last_login();

        let services = get_service_all(&config.systemctl);
        let dockers = get_docker(&config.docker);
        let cpu_temp = get_cpu_temp(&config.cputemp).unwrap_or_default();
        let disks = get_disks(&config.disk);
        let vms = get_vms(&config.vm);
        let gpus = get_gpus(&config.gpu);

        Self { memory, load_avg, cpu_temp, uptime, last_login, disks, services, dockers, vms, gpus }
    }

    // pub fn update(&mut self) -> &mut Self {
    //     // self.sys.refresh_all();
    //     self
    // }
}
