// run  := cargo run --
// dir  := .
// kid  :=

use crate::load_config::Config;
use crate::system_stats::SystemStats;
use crate::utils::{byte2str, s2time};

pub fn to_bold(input: &str) -> String { format!("\x1b[1m{}\x1b[0m", input) }

use std::fmt::Write as FmtWrite;

const PADDING_BEFORE: usize = 2;
const PADDING_AFTER: usize = 15;
const PADDING_INDENT: usize = 3;
const PADDING_MEMORY: usize = 12;

pub fn generate_text(config: &Config, stats: &SystemStats) -> String {
    let mut text = String::new();

    if config.memory {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{}\n{:<padb$}{:<pada$}{}\n{:<padb$}{:<pada$}{}\n{:<padb$}{:<pada$}{}\n{:<padb$}{:<pada$}{}\n{:<padb$}{:<pada$}{}\n{:<padb$}{:<pada$}{} / {}",
            "",
            "Active", byte2str(stats.memory.active_mem, true),
            "",
            "Total", byte2str(stats.memory.total_mem, true),
            "",
            "Available", byte2str(stats.memory.available_mem, true),
            "",
            "Free", byte2str(stats.memory.free_mem, true),
            "",
            "Buffer", byte2str(stats.memory.buffer, true),
            "",
            "Cache", byte2str(stats.memory.cache, true),
            "",
            "Swap",
            byte2str(stats.memory.total_swap - stats.memory.free_swap, true),
            byte2str(stats.memory.total_swap, true),
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER
        )
        .unwrap();
        writeln!(&mut text).unwrap();
    }

    if config.cpuload {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{:.2}%\n{:<padb$}{:<pada$}{:.2}%\n{:<padb$}{:<pada$}{:.2}%",
            "",
            " 1 min",
            stats.load_avg.one,
            "",
            " 5 mins",
            stats.load_avg.five,
            "",
            "15 mins",
            stats.load_avg.fifteen,
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER,
        )
        .unwrap();
        writeln!(&mut text).unwrap();
    }

    if config.cputemp != "none" {
        for (name, temp) in &stats.cpu_temp {
            writeln!(
                &mut text,
                "{:<padb$}{:<pada$}{:2} °C",
                "",
                name,
                *temp as f64 / 1000.0,
                padb = PADDING_BEFORE,
                pada = PADDING_AFTER
            )
            .unwrap();
        }
        writeln!(&mut text).unwrap();
    }

    if config.uptime {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{}\n",
            "",
            "Uptime",
            s2time(stats.uptime),
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER
        )
        .unwrap();
    }

    if config.lastlogin {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{}@{}\n",
            "",
            "Last",
            stats.last_login.user,
            stats.last_login.host,
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER
        )
        .unwrap();
    }

    for disk in &stats.disks {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{} / {}",
            "",
            disk.name,
            byte2str(disk.used, true),
            byte2str(disk.total, true),
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER,
        )
        .unwrap();
        for (subvol_name, subvol_size) in &disk.subvol {
            writeln!(
                &mut text,
                "{:<padb$}{:<padi$}{:<pada$}{}",
                "",
                "",
                subvol_name,
                byte2str(*subvol_size, true),
                padb = PADDING_BEFORE,
                pada = PADDING_AFTER - PADDING_INDENT,
                padi = PADDING_INDENT
            )
            .unwrap();
        }
    }
    if !stats.disks.is_empty() { writeln!(&mut text).unwrap() };

    // Services
    for service in &stats.services {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{:<padm$}{}:{}",
            "",
            service.name,
            byte2str(service.memory, true),
            service.state,
            service.substate,
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER,
            padm = PADDING_MEMORY
        )
        .unwrap();
    }
    if !stats.services.is_empty() { writeln!(&mut text).unwrap() };

    // Dockers
    for docker in &stats.dockers {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{}: {}",
            "",
            docker.name,
            docker.state,
            docker.status,
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER
        )
        .unwrap();
    }
    if !stats.dockers.is_empty() { writeln!(&mut text).unwrap() };

    for vm in &stats.vms {
        writeln!(
            &mut text,
            "{:<padb$}{:<pada$}{}, {} cpu(s), {} / {}, {}",
            "",
            vm.name,
            vm.state,
            vm.cpus,
            byte2str(vm.used_mem, true),
            byte2str(vm.total_mem, true),
            if vm.autostart == "enable" { "autostart" } else { "no autostart" },
            padb = PADDING_BEFORE,
            pada = PADDING_AFTER
        )
        .unwrap();
    }
    if !stats.vms.is_empty() { writeln!(&mut text).unwrap() };
    to_bold(text.as_str())
}

