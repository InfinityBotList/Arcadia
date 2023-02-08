extern crate vergen;
use anyhow::Result;
use vergen::*;

fn main() -> Result<()> {
    let mut config = Config::default();

    *config.git_mut().sha_kind_mut() = ShaKind::Normal;

    *config.git_mut().semver_kind_mut() = SemverKind::Normal;

    *config.git_mut().semver_mut() = true;

    *config.git_mut().semver_dirty_mut() = Some("-dirty");

    vergen(config)
}
