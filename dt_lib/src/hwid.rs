// rust-hwid
// (c) 2020 tilda, under MIT license

//! Get a "Hardware ID" for the host machine. This is a UUID
//! which is intended to uniquely represent this entity.

use thiserror::Error;

/// Possible failure cases for [get_id()].
#[derive(Debug, Error)]
pub enum HwIdError {
    /// Could not detect a hardware id. This might be caused
    /// by a misconfigured system or by this feature not
    /// being supported by the system or platform.
    #[error("no HWID was found on system")]
    NotFound,
    /// Found a putative HWID, but something was wrong with
    /// it.  The `String` argument contains a path or other
    /// identifier at which the HWID was found. This will
    /// usually indicate something is really wrong with the
    /// system.
    #[error("{0:?}: contains malformed HWID")]
    Malformed(String),
}

#[cfg(target_os = "windows")]
mod hwid {
    use super::*;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE};

    /// Get the hardware ID of this machine. The HWID is
    /// obtained from the Windows registry at location
    /// `\\\\SOFTWARE\\Microsoft\\Cryptography\\MachineGuid`.
    pub fn get_id() -> Result<std::string::String, HwIdError> {
        // escaping is fun, right? right???
        let hive = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags("Software\\Microsoft\\Cryptography", KEY_QUERY_VALUE)
            .or(Err(HwIdError::NotFound))?;
        let id = hive.get_value("MachineGuid").or(Err(HwIdError::NotFound))?;
        Ok(id)
    }
}

#[cfg(target_os = "macos")]
mod hwid {
    use super::*;

    /// Get the hardware ID of this machine. The HWID is
    /// obtained by running
    ///
    /// ```text
    /// ioreg -rd1 -c IOExpertPlatformDevice
    /// ```
    ///
    /// and returning the result.
    pub fn get_id() -> Result<std::string::String, HwIdError> {
        let cmd = std::process::Command::new("ioreg")
            .arg("-rd1")
            .arg("-c")
            .arg("IOPlatformExpertDevice")
            .output()
            .or(Err(HwIdError::NotFound))?
            .stdout;
        let out = String::from_utf8(cmd).or(Err(HwIdError::Malformed(String::from("ioreg"))))?;
        match out
            .lines()
            .find(|l| l.contains("IOPlatformUUID"))
            .unwrap_or("")
            .split('=')
            .nth(1)
            .unwrap_or("")
            .split('\"')
            .nth(1)
        {
            None => Err(HwIdError::NotFound),
            Some(id) => {
                if id.is_empty() {
                    Err(HwIdError::Malformed(String::from("ioreg")))
                } else {
                    Ok(id.to_string())
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod hwid {
    use super::*;

    /// Get the hardware ID of this machine. The HWID is
    /// obtained from `/var/lib/dbus/machine-id`, or failing
    /// that from `/etc/machine-id`.
    pub fn get_id() -> Result<std::string::String, HwIdError> {
        let paths = ["/var/lib/dbus/machine-id", "/etc/machine-id"];
        for p in paths {
            if let Ok(id_contents) = std::fs::read_to_string(p) {
                let id_str = id_contents
                    .lines()
                    .next()
                    .ok_or_else(|| HwIdError::Malformed(id_contents.to_string()))?;
                return Ok(id_str.to_string());
            }
        }
        Err(HwIdError::NotFound)
    }
}

#[cfg(target_os = "freebsd")]
#[cfg(target_os = "dragonfly")]
#[cfg(target_os = "openbsd")]
#[cfg(target_os = "netbsd")]
mod hwid {
    pub fn get_id() -> std::string::String {
        unimplemented!("*BSD support is not implemented")
    }
}
pub use hwid::get_id;
