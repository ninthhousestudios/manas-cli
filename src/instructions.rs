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

const USER_FILE_NAME: &str = "CLAUDE.md";
const USER_ENV_OVERRIDE: &str = "MANAS_USER_INSTRUCTIONS";

/// A resolvable instructions file: a compiled-in seed, the stable name it lives
/// under in `~/.manas`, and the env var that overrides its path. The manas
/// instructions and each harness's own addendum are the same shape, so they
/// share one resolver instead of a copy of the seed/read/fallback dance apiece.
struct Spec {
    baked_in: &'static str,
    file_name: &'static str,
    env_override: &'static str,
}

const MANAS: Spec = Spec {
    baked_in: include_str!("adapter/manas-instructions.md"),
    file_name: "manas-instructions.md",
    env_override: "MANAS_INSTRUCTIONS",
};

const CODEX: Spec = Spec {
    baked_in: include_str!("adapter/codex-instructions.md"),
    file_name: "codex-instructions.md",
    env_override: "MANAS_CODEX_INSTRUCTIONS",
};

const GEMINI: Spec = Spec {
    baked_in: include_str!("adapter/gemini-instructions.md"),
    file_name: "gemini-instructions.md",
    env_override: "MANAS_GEMINI_INSTRUCTIONS",
};

const OPENCODE: Spec = Spec {
    baked_in: include_str!("adapter/opencode-instructions.md"),
    file_name: "opencode-instructions.md",
    env_override: "MANAS_OPENCODE_INSTRUCTIONS",
};

const GROK: Spec = Spec {
    baked_in: include_str!("adapter/grok-instructions.md"),
    file_name: "grok-instructions.md",
    env_override: "MANAS_GROK_INSTRUCTIONS",
};

/// A harness that gets its own instructions addendum on top of the shared manas
/// instructions — because its native behaviour needs correcting where Claude's
/// does not (Codex's terse "some bug fixes" commit messages, Grok's Claude-shaped
/// tool names, say).
#[derive(Clone, Copy)]
pub enum Harness {
    Codex,
    Gemini,
    Grok,
    Opencode,
}

impl Harness {
    fn spec(self) -> &'static Spec {
        match self {
            Harness::Codex => &CODEX,
            Harness::Gemini => &GEMINI,
            Harness::Grok => &GROK,
            Harness::Opencode => &OPENCODE,
        }
    }

    /// Map a `manas warm` harness name to its addendum, if it has one. Claude
    /// Code has none — it already writes the commit messages these files exist
    /// to coax out of the others, and its tool names need no translation.
    pub fn from_name(name: &str) -> Option<Harness> {
        match name {
            "codex" => Some(Harness::Codex),
            "gemini" => Some(Harness::Gemini),
            "grok" => Some(Harness::Grok),
            "opencode" | "oc" => Some(Harness::Opencode),
            _ => None,
        }
    }
}

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
    RESOLVED.get_or_init(|| resolve_spec(&MANAS))
}

/// Resolve a harness's addendum on its own — for `manas warm` to report where
/// it resolved from, the same way it reports the manas prompt.
pub fn resolve_harness(harness: Harness) -> Instructions {
    resolve_spec(harness.spec())
}

fn resolve_spec(spec: &Spec) -> Instructions {
    let mut instructions = match std::env::var_os(spec.env_override) {
        Some(raw) => from_env_override(spec, PathBuf::from(raw)),
        None => from_default_path(spec),
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

/// The full on-disk prompt for a harness: the user preamble and manas
/// instructions [`combined`] gives every harness, plus that harness's own
/// addendum appended. Each piece carries its own provenance footer, so a stale
/// session can read exactly which file each part resolved from.
pub fn combined_for(harness: Harness) -> String {
    let addendum = resolve_spec(harness.spec());
    format!("{}\n\n{}", combined().trim_end(), addendum.text)
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

fn from_env_override(spec: &Spec, path: PathBuf) -> Instructions {
    match read_file(&path) {
        Ok(instructions) => instructions,
        Err(e) => {
            eprintln!(
                "  warning:  {}={} unreadable ({e}); using compiled-in instructions",
                spec.env_override,
                path.display()
            );
            baked_in(spec)
        }
    }
}

/// Read `~/.manas/manas-instructions.md`, seeding it from the compiled-in copy
/// if absent — an editable file the user can discover beats an opt-in path they
/// have to be told about.
fn from_default_path(spec: &Spec) -> Instructions {
    let Some(path) = default_path(spec) else {
        eprintln!("  warning:  HOME not set; using compiled-in instructions");
        return baked_in(spec);
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
            baked_in(spec)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => match seed(spec, &path) {
            Ok(instructions) => instructions,
            Err(e) => {
                eprintln!(
                    "  warning:  could not seed {} ({e}); using compiled-in instructions",
                    path.display()
                );
                baked_in(spec)
            }
        },
        Err(e) => {
            eprintln!(
                "  warning:  {} unreadable ({e}); using compiled-in instructions",
                path.display()
            );
            baked_in(spec)
        }
    }
}

fn default_path(spec: &Spec) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".manas").join(spec.file_name))
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

fn seed(spec: &Spec, path: &Path) -> std::io::Result<Instructions> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, spec.baked_in)?;

    let mtime_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());

    Ok(Instructions {
        text: spec.baked_in.to_string(),
        source: Source::File {
            path: path.to_path_buf(),
            seeded: true,
        },
        mtime_epoch,
        hash: fnv1a64(spec.baked_in.as_bytes()),
    })
}

fn baked_in(spec: &Spec) -> Instructions {
    Instructions {
        text: spec.baked_in.to_string(),
        source: Source::BakedIn,
        mtime_epoch: None,
        hash: fnv1a64(spec.baked_in.as_bytes()),
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
        let path = dir.join(MANAS.file_name);
        std::fs::write(&path, "custom instructions\n").unwrap();

        let instructions = from_env_override(&MANAS, path.clone());
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
        let instructions = from_env_override(&MANAS, path);
        assert!(matches!(instructions.source, Source::BakedIn));
        assert_eq!(instructions.text, MANAS.baked_in);
    }

    #[test]
    fn footer_carries_source_and_hash() {
        let instructions = baked_in(&MANAS);
        let footer = instructions.provenance_footer();
        assert!(footer.contains("source=compiled-in"));
        assert!(footer.contains(&format!("fnv1a64={:016x}", instructions.hash)));
    }
}
