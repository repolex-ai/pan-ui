//! pand's configuration, edited as files.
//!
//! Two kinds of file, both per machine (w4r3z, 2026-10-08): `config.yml`,
//! which names the stores, the port and every model pass (model, endpoint,
//! prompt, the settings sent with each request), and the caption prompts in
//! `prompts/`. Stores have none: a soul repo's store is written by pand alone,
//! and a bare store's `pan.yml` is fixed at creation.
//!
//! pan-ui edits these files directly (goodlux, 2026-10-08), not through
//! pand. pand reads them once, at start, so a change takes effect at the
//! next pand restart. A half-written config.yml would stop pand from
//! starting, so a save never writes in place: the old file is copied to
//! `backups/`, the new text goes to a temporary file, and a rename swaps it
//! in, so pand only ever finds the old file or the new one. Before the swap,
//! a new config.yml is run through `pand check-config`, pand's own
//! start-time loader (w4r3z, pan 2862151), so a mistyped key or a prompt
//! that does not exist is refused at save, not discovered at the next start.
//! A `*.default.md` prompt is never written: pand rewrites those itself.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize)]
pub struct ConfigFile {
    /// `config.yml`, or `prompts/<name>.md`.
    pub name: String,
    pub kind: &'static str,
    pub path: String,
    pub text: String,
    /// The file's modification time when read, in milliseconds. A save
    /// sends it back, so an edit made elsewhere in the meantime is not
    /// silently overwritten.
    pub version: u64,
    /// A `*.default.md` prompt is Pan's own and is rewritten when Pan ships
    /// a new one, so an edit to it does not last.
    pub shipped: bool,
    /// The model passes whose `prompt:` names this file.
    pub used_by: Vec<String>,
}

const MASK: &str = "••••••••";

pub fn dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config").join("pan")
}

fn version(p: &Path) -> u64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn read_all(dir: &Path) -> Result<Vec<ConfigFile>, String> {
    let cfg_path = dir.join("config.yml");
    let cfg = std::fs::read_to_string(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let uses = prompt_uses(&cfg);
    let mut out = vec![ConfigFile {
        name: "config.yml".into(),
        kind: "config",
        path: cfg_path.display().to_string(),
        text: mask_secrets(&cfg),
        version: version(&cfg_path),
        shipped: false,
        used_by: vec![],
    }];
    let pdir = dir.join("prompts");
    let mut prompts: Vec<PathBuf> = std::fs::read_dir(&pdir)
        .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    prompts.retain(|p| p.extension().is_some_and(|x| x == "md"));
    prompts.sort();
    for p in prompts {
        let file = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        out.push(ConfigFile {
            name: format!("prompts/{file}"),
            kind: "prompt",
            path: p.display().to_string(),
            version: version(&p),
            text,
            shipped: file.ends_with(".default.md"),
            used_by: uses.iter().filter(|(_, f)| *f == file).map(|(pass, _)| pass.clone()).collect(),
        });
    }
    Ok(out)
}

/// The file a name refers to, or why it is not one pan-ui may write.
/// `config.yml`, or a prompt: `prompts/<letters, digits, . - _>.md`.
pub fn resolve(dir: &Path, name: &str) -> Result<PathBuf, String> {
    if name == "config.yml" {
        return Ok(dir.join("config.yml"));
    }
    let Some(file) = name.strip_prefix("prompts/") else {
        return Err(format!("`{name}` is not config.yml or a prompt"));
    };
    let ok = file.ends_with(".md")
        && file.len() > 3
        && !file.starts_with('.')
        && file.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_');
    if !ok {
        return Err(format!("`{file}` is not a prompt name: letters, digits, '.', '-', '_', ending .md"));
    }
    Ok(dir.join("prompts").join(file))
}

pub enum SaveError {
    /// The text cannot be saved as it is.
    Invalid(String),
    /// Someone changed the file since it was read.
    Changed,
    Io(String),
}

/// Save one file. `base` is the version the editor started from: None to
/// create a file that must not exist yet.
pub fn save(dir: &Path, name: &str, text: &str, base: Option<u64>) -> Result<SavedFile, SaveError> {
    let path = resolve(dir, name).map_err(SaveError::Invalid)?;
    if name.ends_with(".default.md") {
        return Err(SaveError::Invalid(format!(
            "{name} ships with Pan and pand rewrites it when Pan ships a new one; save your version under its own name"
        )));
    }
    let exists = path.exists();
    match (base, exists) {
        (None, true) => return Err(SaveError::Invalid(format!("{name} already exists"))),
        (Some(_), false) => return Err(SaveError::Changed),
        (Some(v), true) if v != version(&path) => return Err(SaveError::Changed),
        _ => {}
    }
    let mut text = text.to_string();
    if name == "config.yml" {
        let current = std::fs::read_to_string(&path).map_err(|e| SaveError::Io(e.to_string()))?;
        text = unmask_secrets(&text, &current).map_err(SaveError::Invalid)?;
        if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&text) {
            return Err(SaveError::Invalid(format!("not valid YAML, so not saved: {e}")));
        }
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let tmp = path.with_extension("pan-ui-tmp");
    std::fs::write(&tmp, &text).map_err(|e| SaveError::Io(format!("{}: {e}", tmp.display())))?;
    let checked = if name == "config.yml" {
        match check(&tmp) {
            // pand names the file it read, the temporary one; say the real name.
            Ok(line) => line.map(|l| l.replace(&tmp.display().to_string(), &path.display().to_string())),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                let e = e.replace(&tmp.display().to_string(), &path.display().to_string());
                return Err(SaveError::Invalid(format!("pand would not start with this, so it was not saved:\n{e}")));
            }
        }
    } else {
        None
    };

    let backup = if exists {
        let stamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S");
        let file = path.file_name().unwrap_or_default().to_string_lossy();
        let b = dir.join("backups").join(format!("{file}.{stamp}"));
        std::fs::create_dir_all(b.parent().unwrap()).map_err(|e| SaveError::Io(e.to_string()))?;
        std::fs::copy(&path, &b).map_err(|e| SaveError::Io(format!("backup {}: {e}", b.display())))?;
        Some(b.display().to_string())
    } else {
        None
    };
    std::fs::rename(&tmp, &path).map_err(|e| SaveError::Io(format!("{}: {e}", path.display())))?;
    Ok(SavedFile { path: path.display().to_string(), version: version(&path), backup, checked })
}

/// Run pand's own loader over a candidate file. Ok(Some(summary)) when pand
/// would start with it; Ok(None) when no pand binary is installed to ask, in
/// which case only the YAML check above has run, and the reply says so.
fn check(file: &Path) -> Result<Option<String>, String> {
    // Unit tests write toy configs pand would rightly refuse; the real check
    // is exercised against pand itself, not here.
    if cfg!(test) {
        return Ok(None);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [PathBuf::from("pand"), PathBuf::from(&home).join(".cargo/bin/pand")];
    for bin in candidates {
        match std::process::Command::new(&bin).arg("check-config").arg(file).output() {
            Ok(out) if out.status.success() => {
                return Ok(Some(String::from_utf8_lossy(&out.stdout).trim().to_string()));
            }
            Ok(out) => {
                let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                return Err(if msg.is_empty() { format!("pand check-config failed ({})", out.status) } else { msg });
            }
            Err(_) => continue,
        }
    }
    Ok(None)
}

#[derive(Serialize)]
pub struct SavedFile {
    pub path: String,
    pub version: u64,
    pub backup: Option<String>,
    /// pand check-config's summary of the saved config, or None when it was
    /// not checked (a prompt, or no pand installed).
    pub checked: Option<String>,
}

/// Which pass names which prompt: a `prompt:` line under a pass inside
/// `models:`. A line reader, not a YAML parser, because only the reading
/// matters here and the file's comments must never be touched.
fn prompt_uses(cfg: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut in_models = false;
    let mut pass: Option<String> = None;
    for line in cfg.lines() {
        let code = line.split(" #").next().unwrap_or("").trim_end();
        if code.trim_start().starts_with('#') || code.trim().is_empty() {
            continue;
        }
        let indent = code.len() - code.trim_start().len();
        let t = code.trim_start();
        if indent == 0 {
            in_models = t.starts_with("models:");
            pass = None;
        } else if in_models && indent == 2 && t.ends_with(':') {
            pass = Some(t.trim_end_matches(':').to_string());
        } else if in_models && indent == 4 {
            if let (Some(p), Some(v)) = (&pass, t.strip_prefix("prompt:")) {
                out.push((p.clone(), v.trim().trim_matches('"').trim_matches('\'').to_string()));
            }
        }
    }
    out
}

fn live_auth(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("auth:") && !t["auth:".len()..].trim().is_empty()
}

/// `auth:` values are credentials. The page never needs them, so they never
/// leave this process; a commented-out line is left as it is.
fn mask_secrets(cfg: &str) -> String {
    let mut out = String::with_capacity(cfg.len());
    for line in cfg.split_inclusive('\n') {
        if live_auth(line) {
            let t = line.trim_start();
            let indent = &line[..line.len() - t.len()];
            let nl = if line.ends_with('\n') { "\n" } else { "" };
            out.push_str(&format!("{indent}auth: {MASK}{nl}"));
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Put the real credentials back: the n-th masked `auth:` line in the
/// edited text takes the value of the n-th live `auth:` line in the file.
/// A typed-in value is kept as typed. More masks than credentials means the
/// text cannot be matched up, and it is refused rather than guessed.
fn unmask_secrets(edited: &str, current: &str) -> Result<String, String> {
    let real: Vec<&str> = current.lines().filter(|l| live_auth(l)).map(|l| l.trim_start()).collect();
    let mut k = 0;
    let mut out = String::with_capacity(edited.len());
    for line in edited.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("auth:") && t["auth:".len()..].trim() == MASK {
            let Some(r) = real.get(k) else {
                return Err("a hidden auth: line has no saved value to restore; type the credential in".into());
            };
            k += 1;
            let indent = &line[..line.len() - t.len()];
            let nl = if line.ends_with('\n') { "\n" } else { "" };
            out.push_str(&format!("{indent}{r}{nl}"));
        } else {
            out.push_str(line);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = "stores:\n  - /a\nmodels:\n  caption:\n    url: http://x\n    # prompt: old.md\n    prompt: full-caption.default.md   # Pan's\n    auth: sk-secret\n  embed:\n    model: e\n    # auth: Bearer <token>\nport: 7401\n";

    #[test]
    fn finds_which_pass_uses_which_prompt() {
        assert_eq!(prompt_uses(CFG), [("caption".to_string(), "full-caption.default.md".to_string())]);
    }

    #[test]
    fn masks_live_credentials_and_puts_them_back() {
        let m = mask_secrets(CFG);
        assert!(!m.contains("sk-secret"));
        assert!(m.contains("    auth: ••••••••\n"));
        assert!(m.contains("# auth: Bearer <token>"));
        assert_eq!(unmask_secrets(&m, CFG).unwrap(), CFG);
        let edited = m.replace("port: 7401", "port: 7402");
        assert_eq!(unmask_secrets(&edited, CFG).unwrap(), CFG.replace("7401", "7402"));
        let extra = format!("{m}  pose:\n    auth: ••••••••\n");
        assert!(unmask_secrets(&extra, CFG).is_err());
    }

    #[test]
    fn only_config_and_prompts() {
        let d = Path::new("/c");
        assert_eq!(resolve(d, "config.yml").unwrap(), Path::new("/c/config.yml"));
        assert_eq!(resolve(d, "prompts/mine.md").unwrap(), Path::new("/c/prompts/mine.md"));
        for bad in ["pan.ttl", "prompts/../config.yml", "prompts/x.txt", "prompts/.md", "prompts/a/b.md", "ontology/pan.ttl"] {
            assert!(resolve(d, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn saves_with_backup_refuses_bad_yaml_and_stale_edits() {
        let d = std::env::temp_dir().join(format!("pan-ui-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("prompts")).unwrap();
        std::fs::write(d.join("config.yml"), CFG).unwrap();
        let files = read_all(&d).unwrap();
        let v = files[0].version;

        assert!(matches!(save(&d, "config.yml", "models: [unclosed", Some(v)), Err(SaveError::Invalid(_))));
        assert_eq!(std::fs::read_to_string(d.join("config.yml")).unwrap(), CFG);

        let s = save(&d, "config.yml", &files[0].text.replace("7401", "7402"), Some(v)).ok().unwrap();
        assert_eq!(std::fs::read_to_string(d.join("config.yml")).unwrap(), CFG.replace("7401", "7402"));
        assert_eq!(std::fs::read_to_string(s.backup.unwrap()).unwrap(), CFG);
        // The editor's copy is now out of date.
        assert!(matches!(save(&d, "config.yml", &files[0].text, Some(v.wrapping_sub(1))), Err(SaveError::Changed)));

        assert!(save(&d, "prompts/mine.md", "hello", None).ok().unwrap().backup.is_none());
        assert!(matches!(save(&d, "prompts/x.default.md", "hi", None), Err(SaveError::Invalid(_))));
        assert!(matches!(save(&d, "prompts/mine.md", "again", None), Err(SaveError::Invalid(_))));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
