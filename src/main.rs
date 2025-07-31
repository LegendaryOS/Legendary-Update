use colored::*;
use std::process::{Command, Stdio};
use std::fs::OpenOptions;
use std::io::Write;
use std::{thread, time};
use chrono::Local;
use regex::Regex;
use sysinfo::System;
use whoami;

const LOGFILE: &str = "/tmp/legendary-update.log";
const FRAMES: [&str; 12] = [
    "[⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯]", "[ ⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯]", "[  ⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯]",
    "[   ⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯]", "[    ⋗⋯⋯⋯⋯⋯⋯⋯⋯]", "[     ⋗⋯⋯⋯⋯⋯⋯⋯]",
    "[      ⋗⋯⋯⋯⋯⋯⋯]", "[       ⋗⋯⋯⋯⋯⋯]", "[        ⋗⋯⋯⋯⋯]",
    "[         ⋗⋯⋯⋯]", "[          ⋗⋯⋯]", "[           ⋗⋯]",
];

fn print_banner() {
    println!(
        "{}",
        r#"
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                LEGENDARY UPDATE                      ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
"#.truecolor(200, 200, 200)
    );
}

fn system_info() {
    let _sys = System::new(); // zmienna nieużywana, prefiks _
    let date = Local::now();
    let user = whoami::username();
    let kernel = System::kernel_version().unwrap_or_else(|| "unknown".to_string());
    let distro = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let pretty = Regex::new(r#"PRETTY_NAME="?(.+?)"?\n"#)
        .unwrap()
        .captures(&distro)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or("unknown".to_string());

    println!("{}", "System Information".bright_blue().bold());
    println!("  Date:    {}", date.format("%Y-%m-%d %H:%M:%S").to_string().blue());
    println!("  User:    {}", user.blue());
    println!("  Kernel:  {}", kernel.blue());
    println!("  Distro:  {}", pretty.blue());
    println!();
}

fn loading_effect(text: &str) {
    for _ in 0..3 {
        print!("{}.", text.bright_cyan());
        thread::sleep(time::Duration::from_millis(300));
        print!(".");
        thread::sleep(time::Duration::from_millis(300));
        println!(".");
    }
}

fn print_status_table(title: &str, status: &str, status_type: &str) {
    let (color, prefix) = match status_type {
        "success" => (Color::Green, "Success: "),
        "error"   => (Color::Red, "Error:   "),
        "warn"    => (Color::Yellow, "Warning: "),
        _         => (Color::BrightBlue, "Info:    "),
    };
    println!("┌──────────────────────────────────────────────────┐");
    println!("│ {:<48} │", title.color(color));
    println!("├──────────────────────────────────────────────────┤");
    println!("│ {:<48} │", format!("{}{}", prefix, status).white());
    println!("└──────────────────────────────────────────────────┘");
    println!();
}

fn run_command(cmd: &str, title: &str) {
    print_status_table(title, &format!("Executing: {}", cmd), "info");

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start command");

    show_progress(&mut child);

    let output = child.wait_with_output().expect("failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut logfile = OpenOptions::new()
        .append(true)
        .create(true)
        .open(LOGFILE)
        .expect("Cannot open logfile");
    writeln!(logfile, "### {} @ {}", title, Local::now()).unwrap();
    writeln!(logfile, "{}", stdout).unwrap();
    writeln!(logfile, "{}", stderr).unwrap();

    if output.status.success() {
        print_status_table(title, "Completed successfully", "success");
    } else {
        print_status_table(title, &format!("Failed - Check log: {}", LOGFILE), "error");
    }
}

fn show_progress(child: &mut std::process::Child) {
    let mut i = 0;
    while let Ok(None) = child.try_wait() {
        print!("\r{}", FRAMES[i].bright_green());
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        i = (i + 1) % FRAMES.len();
        thread::sleep(time::Duration::from_millis(200));
    }
    print!("\r{}\r", " ".repeat(FRAMES[0].len()));
}

fn update_pacman() {
    run_command("sudo pacman -Syu --noconfirm", "Pacman Update");
}

fn update_yay() {
    if which("yay") {
        run_command("yay -Syu --noconfirm", "Yay Update");
    } else {
        print_status_table("Yay Update", "yay not installed - Skipping", "warn");
    }
}

fn update_flatpak() {
    if which("flatpak") {
        run_command("flatpak update -y", "Flatpak Update");
    } else {
        print_status_table("Flatpak Update", "Not installed - Skipping", "warn");
    }
}

fn update_firmware() {
    if which("fwupdmgr") {
        run_command("sudo fwupdmgr update", "Firmware Update");
    } else {
        print_status_table("Firmware Update", "fwupdmgr not installed - Skipping", "warn");
    }
}

fn cleanup_pacman() {
    run_command("sudo pacman -Rns $(pacman -Qdtq)", "Remove Unused Dependencies");
    run_command("sudo pacman -Sc --noconfirm", "Clean Package Cache");
}

fn which(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("which {}", cmd))
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn main() {
    ctrlc::set_handler(|| {
        println!("\n{}", "Interrupted by user".red());
        std::process::exit(130);
    }).expect("Error setting Ctrl-C handler");

    run_command("clear", "Clear Terminal");
    print_banner();
    system_info();
    loading_effect("Preparing update");

    let mut logfile = OpenOptions::new().append(true).create(true).open(LOGFILE).unwrap();
    writeln!(logfile, "\n=== Start: {} ===", Local::now()).unwrap();

    update_pacman();
    update_yay();
    update_flatpak();
    update_firmware();
    cleanup_pacman();

    writeln!(logfile, "=== Completed: {} ===", Local::now()).unwrap();
    print_status_table("System Update", "All tasks completed", "success");
}
