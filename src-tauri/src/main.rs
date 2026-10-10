#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The pipe is held by the GUI only. EOF also occurs after SIGKILL or a crash.
#[cfg(target_os = "macos")]
fn guard_process_group(group: i32) {
    use std::io::Read;
    if group <= 1 || unsafe { libc::geteuid() } == 0 || unsafe { libc::getpgid(group) } != group {
        std::process::exit(2);
    }
    let mut byte = [0u8; 1];
    loop {
        match std::io::stdin().read(&mut byte) {
            Ok(0) => break,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
            Ok(_) => {},
        }
    }
    unsafe { libc::kill(-group, libc::SIGKILL); }
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--guard-process-group") {
            if let Some(group) = args.get(2).and_then(|v| v.parse::<i32>().ok()) {
                guard_process_group(group);
            }
            return;
        }
    }
    aether_desktop_lib::run();
}
