use eyre::{Result, WrapErr, eyre};
use std::path::{Path, PathBuf};
use std::process::Command;

const DODECA_REPO_URL: &str = "https://github.com/bearcove/dodeca";
const DEFAULT_OUTPUT_DIR: &str = "docs/generate";
const TARGET_PROFILE: &str = "release";
const REQUIRED_CELL_SUFFIXES: &[&str] = &[
    "image",
    "webp",
    "jxl",
    "markdown",
    "html",
    "minify",
    "css",
    "sass",
    "js",
    "svgo",
    "fonts",
    "linkcheck",
    "html-diff",
    "dialoguer",
    "code-execution",
    "http",
    "gingembre",
    "data",
    "vite",
    "term",
];

/// `tracey generate` の実体。
/// dodeca(crate) を直接ビルドし、ddc build で静的ページを生成する。
pub fn run(root: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    let project_root = root.unwrap_or_else(|| crate::find_project_root().unwrap_or_default());
    let output_dir = resolve_output_dir(&project_root, output);

    let tools_base = project_root.join(".tracey/tools");
    let source_root = tools_base.join("dodeca-src");
    let ddc_path = ddc_binary_path(&source_root);

    ensure_dodeca_ready(&source_root, &ddc_path)?;
    run_ddc_build(&ddc_path, &project_root, &output_dir)?;
    let rewritten =
        crate::generate_postprocess::rewrite_for_file_scheme(&project_root, &output_dir)?;
    if rewritten > 0 {
        println!("Rewrote {rewritten} generated file(s) for local file:// browsing");
    }

    println!("Generated static site at {}", output_dir.display());
    Ok(())
}

fn resolve_output_dir(project_root: &Path, output: Option<PathBuf>) -> PathBuf {
    match output {
        Some(path) if path.is_absolute() => path,
        Some(path) => project_root.join(path),
        None => project_root.join(DEFAULT_OUTPUT_DIR),
    }
}

fn ddc_binary_path(source_root: &Path) -> PathBuf {
    let bin = source_root.join("target").join(TARGET_PROFILE);
    if cfg!(windows) {
        bin.join("ddc.exe")
    } else {
        bin.join("ddc")
    }
}

fn ensure_dodeca_ready(source_root: &Path, ddc_path: &Path) -> Result<()> {
    ensure_dodeca_source(source_root)?;
    ensure_wasm_pack_installed()?;
    build_dodeca_devtools(source_root)?;

    // 必要セルが不足していると ddc が起動時に失敗するため、ワークスペースの bin を一括ビルドする。
    if !dodeca_runtime_ready(source_root, ddc_path) {
        build_dodeca_workspace_binaries(source_root)?;
    }

    if !dodeca_runtime_ready(source_root, ddc_path) {
        let missing = missing_cell_suffixes(source_root).join(", ");
        return Err(eyre!(
            "dodeca build completed but runtime binaries are still missing.\n\
            ddc: {}\n\
            missing cells: {}",
            ddc_path.display(),
            missing
        ));
    }
    Ok(())
}

fn dodeca_runtime_ready(source_root: &Path, ddc_path: &Path) -> bool {
    ddc_path.exists() && missing_cell_suffixes(source_root).is_empty()
}

fn missing_cell_suffixes(source_root: &Path) -> Vec<String> {
    REQUIRED_CELL_SUFFIXES
        .iter()
        .filter(|suffix| !cell_binary_path(source_root, suffix).exists())
        .map(|suffix| (*suffix).to_string())
        .collect()
}

fn cell_binary_path(source_root: &Path, suffix: &str) -> PathBuf {
    let bin_dir = source_root.join("target").join(TARGET_PROFILE);
    if cfg!(windows) {
        bin_dir.join(format!("ddc-cell-{suffix}.exe"))
    } else {
        bin_dir.join(format!("ddc-cell-{suffix}"))
    }
}

fn run_ddc_build(ddc_path: &Path, project_root: &Path, output_dir: &Path) -> Result<()> {
    // ddc は positional の project path + --output 指定を受け取る。
    let status = Command::new(ddc_path)
        .arg("build")
        .arg(project_root)
        .arg("--output")
        .arg(output_dir)
        .current_dir(project_root)
        .status()
        .wrap_err_with(|| format!("Failed to run ddc at {}", ddc_path.display()))?;

    if !status.success() {
        return Err(eyre!(
            "ddc build failed (exit status: {}) for project {}",
            status,
            project_root.display()
        ));
    }
    Ok(())
}

fn build_dodeca_workspace_binaries(source_root: &Path) -> Result<()> {
    println!("Building dodeca workspace binaries ...");
    let status = Command::new("cargo")
        .arg("build")
        .arg("--locked")
        .arg("--release")
        .arg("--workspace")
        .arg("--bins")
        .current_dir(source_root)
        .status()
        .wrap_err("Failed to run cargo build for dodeca workspace binaries")?;
    if !status.success() {
        return Err(eyre!(
            "Failed to build dodeca workspace binaries (exit status: {})",
            status
        ));
    }
    Ok(())
}

fn ensure_dodeca_source(source_root: &Path) -> Result<()> {
    if source_root.join(".git").exists() {
        return Ok(());
    }

    println!("Cloning dodeca source from {} ...", DODECA_REPO_URL);
    let status = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(DODECA_REPO_URL)
        .arg(source_root)
        .status()
        .wrap_err("Failed to run git clone for dodeca")?;
    if !status.success() {
        return Err(eyre!(
            "Failed to clone dodeca repository (exit status: {})",
            status
        ));
    }
    Ok(())
}

fn ensure_wasm_pack_installed() -> Result<()> {
    let has_wasm_pack = Command::new("wasm-pack")
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if has_wasm_pack {
        return Ok(());
    }

    println!("Installing wasm-pack (required by dodeca build) ...");
    let status = Command::new("cargo")
        .arg("install")
        .arg("--locked")
        .arg("wasm-pack")
        .status()
        .wrap_err("Failed to run cargo install for wasm-pack")?;
    if !status.success() {
        return Err(eyre!(
            "Failed to install wasm-pack (exit status: {})",
            status
        ));
    }
    Ok(())
}

fn build_dodeca_devtools(source_root: &Path) -> Result<()> {
    let dodeca_crate_dir = source_root.join("crates/dodeca");
    let devtools_js = source_root.join("crates/dodeca-devtools/pkg/dodeca_devtools.js");
    let devtools_wasm = source_root.join("crates/dodeca-devtools/pkg/dodeca_devtools_bg.wasm");
    if devtools_js.exists() && devtools_wasm.exists() {
        return Ok(());
    }

    println!("Building dodeca devtools (wasm-pack) ...");
    let status = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("../dodeca-devtools")
        .current_dir(&dodeca_crate_dir)
        .status()
        .wrap_err("Failed to run wasm-pack build for dodeca-devtools")?;
    if !status.success() {
        return Err(eyre!(
            "wasm-pack build failed for dodeca-devtools (exit status: {})",
            status
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_output_dir;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_output_is_docs_generate_under_project_root() {
        let root = Path::new("project-root");
        let got = resolve_output_dir(root, None);
        assert_eq!(got, PathBuf::from("project-root/docs/generate"));
    }

    #[test]
    fn relative_output_is_resolved_from_project_root() {
        let root = Path::new("project-root");
        let got = resolve_output_dir(root, Some(PathBuf::from("site-out")));
        assert_eq!(got, PathBuf::from("project-root/site-out"));
    }
}
