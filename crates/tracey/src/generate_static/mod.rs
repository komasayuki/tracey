use std::fs;
use std::path::Path;

use eyre::{Result, WrapErr};

mod assets;
mod bundle;
mod rendering;
mod sanitize;
mod shim;
mod snapshot;
mod vendor_assets;

pub(crate) async fn generate(project_root: &Path, output_dir: &Path) -> Result<()> {
    prepare_output_dir(output_dir)?;
    let bundle = snapshot::build(project_root, output_dir).await?;
    assets::write_site(output_dir, &bundle)?;
    Ok(())
}

fn prepare_output_dir(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)
            .wrap_err_with(|| format!("Failed to clean output dir {}", output_dir.display()))?;
    }
    fs::create_dir_all(output_dir)
        .wrap_err_with(|| format!("Failed to create output dir {}", output_dir.display()))?;
    Ok(())
}
