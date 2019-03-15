use crate::config::Platform;

pub fn whoami<'a>() -> String {
    let stdout = std::process::Command::new("whoami").output().expect("tried to get username").stdout;
    std::str::from_utf8(&stdout).expect("").trim().into()
}

pub fn hostname() -> String {
    let stdout = std::process::Command::new("uname").args(&["-n"]).output().expect("tried to get username").stdout;
    std::str::from_utf8(&stdout).expect("").trim().into()
}

pub fn platform() -> Platform {
    let stdout = std::process::Command::new("uname").args(&["-o"]).output().expect("tried to get username").stdout;
    let p = std::str::from_utf8(&stdout).expect("").trim();
    match p {
        "GNU/Linux" => Platform::Linux(None),
        _ => Platform::Unknown
    }
}
