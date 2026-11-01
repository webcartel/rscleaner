use std::path::Path;

use rscleaner_platform::{Env, KnownFolder, KnownFolders, PlatformKind};

use crate::error::{GuardError, GuardReason};
use crate::normalize::{path_key, same_or_below, windows_prefix};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Extent {
    SelfOnly,
    Inside,
    Below,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    Forbid,
    Files,
    Allow,
}

#[derive(Clone, Debug)]
struct Range {
    path: String,
    extent: Extent,
    rule: Rule,
}

#[derive(Clone, Debug)]
pub struct ProtectionTable {
    ranges: Vec<Range>,
    platform: PlatformKind,
}

impl ProtectionTable {
    pub fn new(
        env: &dyn Env,
        known: &dyn KnownFolders,
        platform: PlatformKind,
        extra: &[String],
    ) -> Self {
        let mut ranges = match platform {
            PlatformKind::Windows => windows_ranges(env, known),
            PlatformKind::Linux => linux_ranges(env, known),
        };
        for path in extra {
            ranges.push(Range {
                path: path_key(path),
                extent: Extent::Below,
                rule: Rule::Forbid,
            });
        }
        Self { ranges, platform }
    }

    pub fn check(&self, path: &str, filtered: bool) -> Result<(), GuardError> {
        if let Some(range) = self.best_match(path) {
            if !range_permits(range.rule, filtered) {
                return Err(GuardError::new(
                    GuardReason::Protected,
                    format!("{} is protected by {}", path, range.path),
                ));
            }
        }
        Ok(())
    }

    fn best_match(&self, path: &str) -> Option<Range> {
        let key = path_key(path);
        let per_drive = self.per_drive(path);
        let mut best: Option<Range> = None;
        for range in self.ranges.iter().chain(per_drive.iter()) {
            if !covers(range, &key) {
                continue;
            }
            let replace = match &best {
                None => true,
                Some(current) => {
                    range.path.len() > current.path.len()
                        || (range.path.len() == current.path.len() && range.rule < current.rule)
                }
            };
            if replace {
                best = Some(range.clone());
            }
        }
        best
    }

    fn per_drive(&self, path: &str) -> Vec<Range> {
        if self.platform != PlatformKind::Windows {
            return Vec::new();
        }
        let key = path_key(path);
        let Some(colon) = key.find(':') else {
            return Vec::new();
        };
        let root = key[..=colon].to_string();
        let mut ranges = vec![Range {
            path: root.clone(),
            extent: Extent::SelfOnly,
            rule: Rule::Files,
        }];
        for name in [
            "$recycle.bin",
            "system volume information",
            "efi",
            "recovery",
        ] {
            ranges.push(Range {
                path: format!("{root}\\{name}"),
                extent: Extent::Below,
                rule: Rule::Forbid,
            });
        }
        ranges
    }
}

fn covers(range: &Range, key: &str) -> bool {
    match range.extent {
        Extent::SelfOnly => key == range.path,
        Extent::Inside => same_or_below(key, &range.path) && key != range.path,
        Extent::Below => same_or_below(key, &range.path),
    }
}

fn range_permits(rule: Rule, filtered: bool) -> bool {
    match rule {
        Rule::Allow => true,
        Rule::Files => filtered,
        Rule::Forbid => false,
    }
}

fn windows_ranges(env: &dyn Env, known: &dyn KnownFolders) -> Vec<Range> {
    let value = |name: &str, fallback: &str| {
        env.get(name)
            .map(|value| value.to_string_lossy().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| fallback.to_string())
    };
    let system_root = value("SYSTEMROOT", r"C:\Windows");
    let profile = value("USERPROFILE", "");
    let program_files = value("PROGRAMFILES", r"C:\Program Files");
    let program_files_x86 = value("PROGRAMFILES(X86)", r"C:\Program Files (x86)");
    let program_data = value("PROGRAMDATA", r"C:\ProgramData");

    let mut ranges = vec![
        range(&program_files, Extent::SelfOnly, Rule::Files),
        range(&program_files_x86, Extent::SelfOnly, Rule::Files),
        range(&program_data, Extent::SelfOnly, Rule::Files),
        range(&system_root, Extent::Below, Rule::Forbid),
        range(r"C:\Users", Extent::Below, Rule::Forbid),
        range(
            &format!("{system_root}\\System32"),
            Extent::Below,
            Rule::Forbid,
        ),
        range(
            &format!("{system_root}\\SysWOW64"),
            Extent::Below,
            Rule::Forbid,
        ),
        range(
            &format!("{system_root}\\WinSxS"),
            Extent::Below,
            Rule::Forbid,
        ),
        range(&format!("{system_root}\\Boot"), Extent::Below, Rule::Forbid),
        range(
            &format!("{system_root}\\Fonts"),
            Extent::Below,
            Rule::Forbid,
        ),
        range(
            &format!("{system_root}\\Recovery"),
            Extent::Below,
            Rule::Forbid,
        ),
        range(&program_files, Extent::Inside, Rule::Forbid),
        range(&program_files_x86, Extent::Inside, Rule::Forbid),
        range(&format!("{system_root}\\Temp"), Extent::Below, Rule::Allow),
        range(&format!("{system_root}\\Logs"), Extent::Below, Rule::Allow),
        range(
            &format!("{system_root}\\Minidump"),
            Extent::Below,
            Rule::Allow,
        ),
        range(
            &format!("{system_root}\\SoftwareDistribution\\Download"),
            Extent::Below,
            Rule::Allow,
        ),
        range(
            &format!(
                "{system_root}\\ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization"
            ),
            Extent::Below,
            Rule::Allow,
        ),
        range(
            &format!("{system_root}\\MEMORY.DMP"),
            Extent::SelfOnly,
            Rule::Allow,
        ),
    ];

    if !profile.is_empty() {
        ranges.push(range(&profile, Extent::SelfOnly, Rule::Files));
        ranges.push(range(&profile, Extent::Below, Rule::Allow));
        for name in [
            "Desktop",
            "Documents",
            "Pictures",
            "Videos",
            "Music",
            "Downloads",
            "OneDrive",
        ] {
            ranges.push(range(
                &format!("{profile}\\{name}"),
                Extent::Below,
                Rule::Files,
            ));
        }
        ranges.push(range(
            &value("APPDATA", &format!("{profile}\\AppData\\Roaming")),
            Extent::SelfOnly,
            Rule::Files,
        ));
        ranges.push(range(
            &value("LOCALAPPDATA", &format!("{profile}\\AppData\\Local")),
            Extent::SelfOnly,
            Rule::Files,
        ));
    }
    if let Some(one_drive) = env.get("ONEDRIVE") {
        ranges.push(range(
            &one_drive.to_string_lossy(),
            Extent::Below,
            Rule::Files,
        ));
    }

    // The known-folder trait is part of the table contract even though the
    // Windows user directories above already cover the standard names.
    let _ = known;
    ranges
}

fn linux_ranges(env: &dyn Env, known: &dyn KnownFolders) -> Vec<Range> {
    let home = env
        .get("HOME")
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut ranges = vec![
        range("/", Extent::Below, Rule::Forbid),
        range("/home", Extent::Below, Rule::Forbid),
        range("/bin", Extent::Below, Rule::Forbid),
        range("/sbin", Extent::Below, Rule::Forbid),
        range("/lib", Extent::Below, Rule::Forbid),
        range("/lib64", Extent::Below, Rule::Forbid),
        range("/usr", Extent::Below, Rule::Forbid),
        range("/boot", Extent::Below, Rule::Forbid),
        range("/dev", Extent::Below, Rule::Forbid),
        range("/proc", Extent::Below, Rule::Forbid),
        range("/sys", Extent::Below, Rule::Forbid),
        range("/run", Extent::Below, Rule::Forbid),
        range("/etc", Extent::Below, Rule::Forbid),
        range("/opt", Extent::Below, Rule::Forbid),
        range("/srv", Extent::Below, Rule::Forbid),
        range("/mnt", Extent::Below, Rule::Forbid),
        range("/media", Extent::Below, Rule::Forbid),
        range("/snap", Extent::Below, Rule::Forbid),
        range("/root", Extent::Below, Rule::Forbid),
        range("/var/lib", Extent::Below, Rule::Forbid),
        range("/tmp", Extent::Below, Rule::Files),
        range("/var/tmp", Extent::Below, Rule::Files),
        range("/var/log", Extent::Below, Rule::Files),
        range("/var/cache/apt/archives", Extent::Below, Rule::Allow),
        range("/var/cache/dnf", Extent::Below, Rule::Allow),
        range("/var/cache/pacman/pkg", Extent::Below, Rule::Allow),
        range("/var/cache/zypp/packages", Extent::Below, Rule::Allow),
        range("/var/crash", Extent::Below, Rule::Allow),
        range("/var/lib/systemd/coredump", Extent::Below, Rule::Allow),
    ];
    if !home.is_empty() {
        ranges.push(range(&home, Extent::SelfOnly, Rule::Files));
        ranges.push(range(&home, Extent::Below, Rule::Allow));
        ranges.push(range(
            &format!("{home}/.config"),
            Extent::SelfOnly,
            Rule::Files,
        ));
        ranges.push(range(
            &format!("{home}/.local"),
            Extent::SelfOnly,
            Rule::Files,
        ));
        ranges.push(range(
            &format!("{home}/.local/share"),
            Extent::SelfOnly,
            Rule::Files,
        ));
        ranges.push(range(
            &format!("{home}/.cache"),
            Extent::SelfOnly,
            Rule::Files,
        ));
        for folder in [
            KnownFolder::Downloads,
            KnownFolder::Documents,
            KnownFolder::Desktop,
            KnownFolder::Pictures,
            KnownFolder::Videos,
            KnownFolder::Music,
        ] {
            if let Some(path) = known.get(folder) {
                ranges.push(range(&path.to_string_lossy(), Extent::Below, Rule::Files));
            }
        }
        for name in [".ssh", ".gnupg", ".password-store", ".local/share/keyrings"] {
            ranges.push(range(
                &format!("{home}/{name}"),
                Extent::Below,
                Rule::Forbid,
            ));
        }
        ranges.push(range(
            &format!("{home}/.local/share/Trash"),
            Extent::Below,
            Rule::Allow,
        ));
    }
    ranges
}

fn range(path: &str, extent: Extent, rule: Rule) -> Range {
    let path = if cfg!(windows) {
        windows_prefix(path)
    } else {
        path.to_string()
    };
    Range {
        path: path_key(&path),
        extent,
        rule,
    }
}

pub fn browser_profile_allows(path: &Path) -> bool {
    let lower = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('\\', "/");
    let in_profile = [
        "user data/",
        "google-chrome/",
        "chromium/",
        "bravesoftware/",
        "microsoft-edge/",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if !in_profile {
        return true;
    }
    let allowed = [
        "/cache",
        "/code cache",
        "/gpucache",
        "/dawncache",
        "/grshadercache",
        "/shadercache",
        "/service worker/cachestorage",
        "/cache2",
    ];
    allowed
        .iter()
        .any(|suffix| lower.ends_with(suffix) || lower.contains(&format!("{suffix}/")))
}
