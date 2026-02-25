use std::path::{Path, PathBuf};

/// ソース表示用の相対パスを求める。
/// プロジェクト外のファイルは `../` を含む相対表現にする。
pub(super) fn display_path(project_root: &Path, full_path: &Path) -> String {
    let root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let path = full_path
        .canonicalize()
        .unwrap_or_else(|_| full_path.to_path_buf());

    if let Ok(rel) = path.strip_prefix(&root) {
        rel.display().to_string()
    } else {
        compute_relative_path(&root, &path)
    }
}

fn compute_relative_path(from: &Path, to: &Path) -> String {
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    let mut common_len = 0;
    for (a, b) in from_components.iter().zip(to_components.iter()) {
        if a == b {
            common_len += 1;
        } else {
            break;
        }
    }

    let mut result = PathBuf::new();
    for _ in common_len..from_components.len() {
        result.push("..");
    }
    for component in &to_components[common_len..] {
        result.push(component);
    }

    result.display().to_string()
}

/// API互換のため、サーバー版と同じエスケープ規則を使う。
pub(super) fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// `arborium` の言語名を拡張子から判定する。
pub(super) fn arborium_language(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "rs" => Some("rust"),
        "go" => Some("go"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some("cpp"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "ts" | "mts" | "cts" => Some("typescript"),
        "jsx" => Some("javascript"),
        "tsx" => Some("tsx"),
        "py" => Some("python"),
        "rb" => Some("ruby"),
        "java" => Some("java"),
        "kt" | "kts" => Some("kotlin"),
        "scala" => Some("scala"),
        "sh" | "bash" | "zsh" => Some("bash"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        "xml" => Some("xml"),
        "html" | "htm" => Some("html"),
        "css" => Some("css"),
        "scss" | "sass" => Some("scss"),
        "md" | "markdown" => Some("markdown"),
        "sql" => Some("sql"),
        "zig" => Some("zig"),
        "swift" => Some("swift"),
        "ex" | "exs" => Some("elixir"),
        "hs" | "lhs" => Some("haskell"),
        "ml" | "mli" => Some("ocaml"),
        "lua" => Some("lua"),
        "php" => Some("php"),
        "r" | "R" => Some("r"),
        "dart" => Some("dart"),
        "asm" | "s" | "S" => Some("asm"),
        "pl" | "pm" => Some("perl"),
        "erl" | "hrl" => Some("erlang"),
        "clj" | "cljs" | "cljc" | "edn" => Some("clojure"),
        "fs" | "fsi" | "fsx" => Some("fsharp"),
        "vb" | "vbs" => Some("vb"),
        "cob" | "cbl" | "cpy" => Some("cobol"),
        "jl" => Some("julia"),
        "d" => Some("d"),
        "ps1" | "psm1" | "psd1" => Some("powershell"),
        "cmake" => Some("cmake"),
        "mat" => Some("matlab"),
        _ => None,
    }
}
