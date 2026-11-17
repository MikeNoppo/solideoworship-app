mod install;
mod release;

use std::fmt;
use std::path::PathBuf;

pub use install::{Handoff, Installation, Staged, run_helper};
pub use release::{ReleaseUpdate, check};

const SUMS: &str = "SHA256SUMS.txt";
const SIGNATURE: &str = "SHA256SUMS.txt.sig";
const MAX_PACKAGE: u64 = 512 * 1024 * 1024;
const MAX_SUMS: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Network,
    Metadata,
    Verification,
    Busy,
    Install,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Network => "Could not contact GitHub Releases",
            Self::Metadata => "Invalid release metadata",
            Self::Verification => "Update package verification failed",
            Self::Busy => "Another update is already in progress",
            Self::Install => "Could not install the update",
        })
    }
}

#[derive(Debug, Default)]
pub struct Restart {
    pub failed: bool,
    pub schedule: Option<PathBuf>,
}

impl Restart {
    pub fn from_args() -> Option<Self> {
        let mut args = std::env::args_os().skip(1);
        let flag = args.next()?;
        let failed = match flag.to_str()? {
            "--update-installed" => false,
            "--update-failed" => true,
            _ => return None,
        };
        Some(Self {
            failed,
            schedule: args.next().map(PathBuf::from),
        })
    }
}

fn public_key() -> Option<[u8; 32]> {
    hex_bytes(option_env!("SDW_UPDATE_PUBLIC_KEY")?)
}

fn hex_bytes(input: &str) -> Option<[u8; 32]> {
    if input.len() != 64 || !input.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&input[index * 2..][..2], 16).ok()?;
    }
    Some(bytes)
}

fn package_name(version: &str) -> String {
    let suffix = if cfg!(windows) {
        "windows-x64-setup.exe"
    } else {
        "linux-x86_64.tar.gz"
    };
    format!("SolideoWorship-{version}-{suffix}")
}
