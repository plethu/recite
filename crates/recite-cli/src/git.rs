//! Read project files at a git revision, for commands whose baseline is a
//! release tag rather than a file the author keeps.
//!
//! The baseline is read from git rather than a sidecar so source stays the only
//! authority. A missing revision (for example, a shallow clone without tags) is
//! an error rather than an empty baseline.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The repository root containing `anchor`, after checking `reference`
/// names a commit there.
pub(crate) fn repository_at(anchor: &Path, reference: &str) -> Result<PathBuf, String> {
    let root = git_text(anchor, &["rev-parse", "--show-toplevel"], None)?
        .trim_end()
        .to_owned();
    // `--show-toplevel` resolves symbolic links; resolve the same way so
    // `pathspec` can strip this prefix from canonical paths.
    let root = std::fs::canonicalize(&root).unwrap_or_else(|_| PathBuf::from(root));
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

/// `path` relative to the repository `root`, for use as a git pathspec and as
/// a path inside a checkout of the repository. The root itself is `.`.
pub(crate) fn pathspec(root: &Path, path: &Path) -> Result<String, String> {
    // Canonical paths have no `..` or symbolic links, so the relative path is
    // also valid inside a checkout that contains only the listed files.
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve {}: {error}", path.display()))?;
    let relative = canonical
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside the git repository", path.display()))?;
    if relative.as_os_str().is_empty() {
        return Ok(".".to_owned());
    }
    Ok(relative.to_string_lossy().into_owned())
}

/// Files under `pathspecs` at `reference` whose repository path `keep`
/// accepts, as `(repository path, contents)`. Only accepted files are read.
pub(crate) fn files_at(
    root: &Path,
    reference: &str,
    pathspecs: &[String],
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut files = Vec::new();
    for_each_file_at(root, reference, pathspecs, keep, |path, contents| {
        files.push((path.to_owned(), contents.to_vec()));
        Ok(())
    })?;
    Ok(files)
}

/// Passes each file under `pathspecs` at `reference` whose repository path
/// `keep` accepts to `each`, one at a time, so only one file is held in
/// memory. Returns how many files were passed.
pub(crate) fn for_each_file_at(
    root: &Path,
    reference: &str,
    pathspecs: &[String],
    keep: impl Fn(&str) -> bool,
    mut each: impl FnMut(&str, &[u8]) -> Result<(), String>,
) -> Result<usize, String> {
    let mut listing_args = vec!["ls-tree", "-r", "-z", reference, "--"];
    listing_args.extend(pathspecs.iter().map(String::as_str));
    let listing = git_text(root, &listing_args, None)?;
    // Entries are `<mode> <type> <object>\t<path>`. Only blobs have contents;
    // submodule gitlinks are not part of this repository's tree.
    let files = listing
        .split('\0')
        .filter_map(|entry| {
            let (header, path) = entry.split_once('\t')?;
            (header.split(' ').nth(1) == Some("blob") && keep(path)).then(|| path.to_owned())
        })
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Ok(0);
    }
    let requests = files
        .iter()
        .map(|file| format!("{reference}:{file}\n"))
        .collect::<String>();
    git_streaming(
        root,
        &["cat-file", "--batch"],
        Some(requests.as_bytes()),
        |output| {
            let mut contents = Vec::new();
            for file in &files {
                read_batch_object(output, &mut contents)?;
                each(file, &contents)?;
            }
            let mut rest = [0; 1];
            if output
                .read(&mut rest)
                .map_err(|error| format!("could not read git output: {error}"))?
                != 0
            {
                return Err("git returned more files than it listed".to_owned());
            }
            Ok(())
        },
    )?;
    Ok(files.len())
}

/// Reads one `git cat-file --batch` object, `<oid> <type> <size>\n<content>\n`,
/// into `contents`.
fn read_batch_object(output: &mut dyn BufRead, contents: &mut Vec<u8>) -> Result<(), String> {
    let read_error = |error: std::io::Error| format!("could not read git output: {error}");
    let mut header = String::new();
    if output.read_line(&mut header).map_err(read_error)? == 0 {
        return Err("git returned fewer files than it listed".to_owned());
    }
    let header = header.trim_end();
    let size = header
        .rsplit(' ')
        .next()
        .and_then(|size| size.parse::<usize>().ok())
        .ok_or_else(|| format!("unexpected git object header `{header}`"))?;
    contents.clear();
    contents.resize(size + 1, 0);
    output
        .read_exact(contents)
        .map_err(|_| "truncated git object".to_owned())?;
    contents.truncate(size);
    Ok(())
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
    let mut stdout = Vec::new();
    git_streaming(directory, args, input, |output| {
        output
            .read_to_end(&mut stdout)
            .map(|_| ())
            .map_err(|error| format!("could not read git output: {error}"))
    })?;
    Ok(stdout)
}

/// Runs git, passing its output to `read` as it arrives.
fn git_streaming(
    directory: &Path,
    args: &[&str],
    input: Option<&[u8]>,
    read: impl FnOnce(&mut dyn BufRead) -> Result<(), String>,
) -> Result<(), String> {
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
    let read_result = match child.stdout.take() {
        Some(stdout) => read(&mut BufReader::new(stdout)),
        None => Ok(()),
    };
    if read_result.is_err() {
        // Stop git so the writer thread and the wait below cannot block.
        let _ = child.kill();
    }
    let written = writer.map(|writer| {
        writer
            .join()
            .map_err(|_| "git input writer panicked".to_owned())
    });
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not run git: {error}"))?;
    read_result?;
    if let Some(written) = written {
        written?.map_err(|error| format!("could not write to git: {error}"))?;
    }
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}
