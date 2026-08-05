//! Resolution of the manas system-prompt instructions.
//!
//! The instructions used to be `include_str!`'d straight into the injected
//! system prompt, which froze them at compile time: editing the source `.md`
//! had no effect until the binary was rebuilt, silently. Resolution now happens
//! at runtime against an editable file, with the compiled-in copy as a
//! seed/fallback so a fresh install still works.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const BAKED_IN: &str = include_str!("adapter/manas-instructions.md");
const FILE_NAME: &str = "manas-instructions.md";
const ENV_OVERRIDE: &str = "MANAS_INSTRUCTIONS";
const USER_FILE_NAME: &str = "CLAUDE.md";
const USER_ENV_OVERRIDE: &str = "MANAS_USER_INSTRUCTIONS";

/// Where the resolved instructions came from.
pub enum Source {
    /// Read from a file on disk. `seeded` means the file did not exist and we
    /// just wrote the compiled-in copy to it.
    File { path: PathBuf, seeded: bool },
    /// The compiled-in copy, because no file was usable.
    BakedIn,
}

pub struct Instructions {
    /// Exactly what gets injected and written to the session scratch dir,
    /// provenance footer included.
    pub text: String,
    pub source: Source,
    /// mtime of the source file, as seconds since the unix epoch.
    pub mtime_epoch: Option<u64>,
    pub hash: u64,
}

impl Instructions {
    /// One-line summary for `manas warm` output: which source won.
    pub fn summary(&self) -> String {
        match &self.source {
            Source::File { path, seeded } => {
                let seeded = if *seeded { " (seeded)" } else { "" };
                let mtime = self
                    .mtime_epoch
                    .map(|m| format!(" mtime={m}"))
                    .unwrap_or_default();
                format!("{}{} #{:016x}{}", path.display(), seeded, self.hash, mtime)
            }
            Source::BakedIn => format!("compiled-in #{:016x}", self.hash),
        }
    }

    fn source_path(&self) -> &str {
        match &self.source {
            Source::File { path, .. } => path.to_str().unwrap_or("<non-utf8 path>"),
            Source::BakedIn => "compiled-in",
        }
    }

    /// Provenance the running session can read off its own system prompt
    /// instead of diffing against a source it has to guess at.
    fn provenance_footer(&self) -> String {
        let mtime = match self.mtime_epoch {
            Some(m) => m.to_string(),
            None => "n/a".to_string(),
        };
        format!(
            "\n<!-- manas-instructions provenance: source={} mtime_epoch={} fnv1a64={:016x} -->\n",
            self.source_path(),
            mtime,
            self.hash,
        )
    }
}

/// Resolve the instructions once per process.
///
/// Infallible by design: every failure path degrades to the compiled-in copy
/// with a warning on stderr, because a missing override should not stop a
/// session from booting.
pub fn resolve() -> &'static Instructions {
    static RESOLVED: OnceLock<Instructions> = OnceLock::new();
    RESOLVED.get_or_init(resolve_uncached)
}

fn resolve_uncached() -> Instructions {
    let mut instructions = match std::env::var_os(ENV_OVERRIDE) {
        Some(raw) => from_env_override(PathBuf::from(raw)),
        None => from_default_path(),
    };
    instructions
        .text
        .push_str(&instructions.provenance_footer());
    instructions
}

/// The whole prompt for harnesses that have no system-prompt flag and can only
/// be reached through an on-disk conventions file (Codex's `AGENTS.md`).
///
/// Claude Code gets these as two independent channels: it discovers
/// `~/CLAUDE.md` itself by walking parent directories, and takes the manas
/// instructions via `--append-system-prompt-file`. Codex does neither, so both
/// have to arrive concatenated or a codex session runs with neither.
pub fn combined() -> &'static str {
    static COMBINED: OnceLock<String> = OnceLock::new();
    COMBINED.get_or_init(|| match user_preamble() {
        Some(preamble) => format!("{}\n\n{}", preamble.trim_end(), resolve().text),
        None => resolve().text.clone(),
    })
}

/// The user's own global instructions, carrying the same kind of provenance
/// header the manas instructions carry — a session that ends up with a stale
/// preamble should be able to read where it came from off its own prompt.
fn user_preamble() -> Option<String> {
    let path = match std::env::var_os(USER_ENV_OVERRIDE) {
        Some(raw) => PathBuf::from(raw),
        None => PathBuf::from(std::env::var_os("HOME")?).join(USER_FILE_NAME),
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            eprintln!(
                "  warning:  {} unreadable ({e}); continuing without the user preamble",
                path.display()
            );
            return None;
        }
    };

    let source = std::fs::canonicalize(&path).unwrap_or(path);
    Some(format!(
        "<!-- manas user preamble: source={} fnv1a64={:016x} -->\n{}",
        source.display(),
        fnv1a64(text.as_bytes()),
        text,
    ))
}

fn from_env_override(path: PathBuf) -> Instructions {
    match read_file(&path) {
        Ok(instructions) => instructions,
        Err(e) => {
            eprintln!(
                "  warning:  {ENV_OVERRIDE}={} unreadable ({e}); using compiled-in instructions",
                path.display()
            );
            baked_in()
        }
    }
}

/// Read `~/.manas/manas-instructions.md`, seeding it from the compiled-in copy
/// if absent — an editable file the user can discover beats an opt-in path they
/// have to be told about.
fn from_default_path() -> Instructions {
    let Some(path) = default_path() else {
        eprintln!("  warning:  HOME not set; using compiled-in instructions");
        return baked_in();
    };

    match read_file(&path) {
        Ok(instructions) => instructions,
        // A dangling symlink also reads as NotFound, but seeding it would
        // follow the link and write through to wherever it points — usually a
        // checkout that has moved. Report it instead.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && is_symlink(&path) => {
            eprintln!(
                "  warning:  {} is a broken symlink; using compiled-in instructions",
                path.display()
            );
            baked_in()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => match seed(&path) {
            Ok(instructions) => instructions,
            Err(e) => {
                eprintln!(
                    "  warning:  could not seed {} ({e}); using compiled-in instructions",
                    path.display()
                );
                baked_in()
            }
        },
        Err(e) => {
            eprintln!(
                "  warning:  {} unreadable ({e}); using compiled-in instructions",
                path.display()
            );
            baked_in()
        }
    }
}

fn default_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".manas").join(FILE_NAME))
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

fn read_file(path: &Path) -> std::io::Result<Instructions> {
    let text = std::fs::read_to_string(path)?;
    let mtime_epoch = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Report the real target: `~/.manas/manas-instructions.md` is expected to
    // be a symlink into a checkout, and naming the link instead of the file it
    // resolves to is the ambiguity this whole task exists to remove.
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    Ok(Instructions {
        hash: fnv1a64(text.as_bytes()),
        text,
        source: Source::File {
            path,
            seeded: false,
        },
        mtime_epoch,
    })
}

fn seed(path: &Path) -> std::io::Result<Instructions> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, BAKED_IN)?;

    let mtime_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());

    Ok(Instructions {
        text: BAKED_IN.to_string(),
        source: Source::File {
            path: path.to_path_buf(),
            seeded: true,
        },
        mtime_epoch,
        hash: fnv1a64(BAKED_IN.as_bytes()),
    })
}

fn baked_in() -> Instructions {
    Instructions {
        text: BAKED_IN.to_string(),
        source: Source::BakedIn,
        mtime_epoch: None,
        hash: fnv1a64(BAKED_IN.as_bytes()),
    }
}

/// FNV-1a, 64-bit. Not cryptographic — this only has to change when the file
/// changes, and it has to stay stable across builds (which `DefaultHasher`
/// does not promise).
fn fnv1a64(data: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_reads_the_file() {
        let dir = std::env::temp_dir().join(format!("manas-instr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        std::fs::write(&path, "custom instructions\n").unwrap();

        let instructions = from_env_override(path.clone());
        assert!(instructions.text.contains("custom instructions"));
        assert!(matches!(
            instructions.source,
            Source::File { seeded: false, .. }
        ));
        assert_eq!(instructions.hash, fnv1a64(b"custom instructions\n"));

        std::fs::write(&path, "").unwrap();
    }

    #[test]
    fn missing_env_override_falls_back_to_baked_in() {
        let path = std::env::temp_dir().join("manas-instr-does-not-exist.md");
        let instructions = from_env_override(path);
        assert!(matches!(instructions.source, Source::BakedIn));
        assert_eq!(instructions.text, BAKED_IN);
    }

    #[test]
    fn footer_carries_source_and_hash() {
        let instructions = baked_in();
        let footer = instructions.provenance_footer();
        assert!(footer.contains("source=compiled-in"));
        assert!(footer.contains(&format!("fnv1a64={:016x}", instructions.hash)));
    }
}
