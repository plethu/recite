use crate::Result;
use semver::Version;

pub(super) fn parse(text: &str) -> Result<Version> {
    let version = Version::parse(text)?;
    if !version.build.is_empty() {
        return Err("release versions cannot differ only by build metadata".into());
    }
    if !version.pre.is_empty() {
        let parts: Vec<_> = version.pre.as_str().split('.').collect();
        if parts.len() != 2
            || !matches!(parts[0], "alpha" | "beta" | "rc")
            || parts[1].parse::<u64>().is_err()
            || parts[1] == "0"
        {
            return Err("use a numbered alpha.N, beta.N or rc.N prerelease (N >= 1)".into());
        }
    }
    Ok(version)
}
