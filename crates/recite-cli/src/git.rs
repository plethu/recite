//! Read project files at a git revision, for commands whose baseline is a
//! release tag rather than a file the author keeps.
//!
//! The baseline is read from git rather than a sidecar so source stays the only
//! authority. A missing revision (for example, a shallow clone without tags) is
//! an error rather than an empty baseline.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The repository root containing `anchor`, after checking `reference`
/// names a commit there.
pub(crate) fn repository_at(anchor: &Path, reference: &str) -> Result<PathBuf, String> {
    let root = git_text(anchor, &["rev-parse", "--show-toplevel"], None)?
        .trim_end()
        .to_owned();
    let root = PathBuf::from(root);
    git(
        &root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
        None,
    )
    .map_err(|_| {
        "the revision is not available; fetch it (and its tags) before checking".to_owned()
    })?;
    Ok(root)
}

/// `path` relative to the repository `root`, for use as a git pathspec.
pub(crate) fn pathspec(root: &Path, path: &Path) -> Result<String, String> {
    let absolute = std::path::absolute(path).map_err(|error| error.to_string())?;
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside the git repository", path.display()))?;
    Ok(relative.to_string_lossy().into_owned())
}

/// Files under `pathspecs` at `reference`, as `(repository path, contents)`.
pub(crate) fn files_at(
    root: &Path,
    reference: &str,
    pathspecs: &[String],
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut listing_args = vec!["ls-tree", "-r", "-z", "--name-only", reference, "--"];
    listing_args.extend(pathspecs.iter().map(String::as_str));
    let listing = git_text(root, &listing_args, None)?;
    let files = listing
        .split('\0')
        .filter(|file| !file.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let requests = files
        .iter()
        .map(|file| format!("{reference}:{file}\n"))
        .collect::<String>();
    let blobs = git(root, &["cat-file", "--batch"], Some(requests.as_bytes()))?;
    let contents = batch_contents(&blobs)?;
    if contents.len() != files.len() {
        return Err("git returned a different number of files than it listed".to_owned());
    }
    Ok(files
        .into_iter()
        .zip(contents.into_iter().map(<[u8]>::to_vec))
        .collect())
}

/// Splits `git cat-file --batch` output: `<oid> <type> <size>\n<content>\n`.
fn batch_contents(output: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut contents = Vec::new();
    let mut rest = output;
    while !rest.is_empty() {
        let newline = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| "truncated git object listing".to_owned())?;
        let header = String::from_utf8_lossy(&rest[..newline]);
        let size = header
            .rsplit(' ')
            .next()
            .and_then(|size| size.parse::<usize>().ok())
            .ok_or_else(|| format!("unexpected git object header `{header}`"))?;
        let after = &rest[newline + 1..];
        let content = after
            .get(..size)
            .ok_or_else(|| "truncated git object".to_owned())?;
        contents.push(content);
        rest = after.get(size + 1..).unwrap_or_default();
    }
    Ok(contents)
}

pub(crate) fn git_text(
    directory: &Path,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<String, String> {
    String::from_utf8(git(directory, args, input)?)
        .map_err(|_| "git output is not valid UTF-8".to_owned())
}

pub(crate) fn git(
    directory: &Path,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not run git: {error}"))?;
    // Feed stdin from its own thread so a large batch cannot fill both pipes.
    let writer = input.zip(child.stdin.take()).map(|(input, mut stdin)| {
        let input = input.to_owned();
        std::thread::spawn(move || stdin.write_all(&input))
    });
    let mut stdout = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout)
            .map_err(|error| format!("could not read git output: {error}"))?;
    }
    if let Some(writer) = writer {
        writer
            .join()
            .map_err(|_| "git input writer panicked".to_owned())?
            .map_err(|error| format!("could not write to git: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}
