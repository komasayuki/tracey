use facet::Facet;
use tracey_api::{ApiConfig, ApiFileData, ApiReverseData, ApiSpecData, ApiSpecForward};

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticManifest {
    pub version: u64,
    pub config: ApiConfig,
    pub health: StaticHealth,
    pub entries: Vec<StaticEntryManifest>,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticHealth {
    #[facet(rename = "configError")]
    pub config_error: Option<String>,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticEntryManifest {
    pub spec: String,
    #[facet(rename = "impl")]
    pub impl_name: String,
    pub chunk: String,
}

#[derive(Debug, Clone)]
pub(super) struct StaticBundle {
    pub manifest: StaticManifest,
    pub entry_chunks: Vec<StaticChunk<StaticEntryChunk>>,
    pub file_chunks: Vec<StaticChunk<StaticFileChunk>>,
}

#[derive(Debug, Clone)]
pub(super) struct StaticChunk<T> {
    pub relative_path: String,
    pub data: T,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticEntryChunk {
    pub spec: String,
    #[facet(rename = "impl")]
    pub impl_name: String,
    pub forward: ApiSpecForward,
    pub reverse: ApiReverseData,
    pub spec_content: ApiSpecData,
    pub files: Vec<StaticFileRef>,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticFileRef {
    pub path: String,
    pub chunk: String,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticFileChunk {
    pub path: String,
    pub data: ApiFileData,
}

/// 静的 chunk の保存先は spec/impl ごとに安定した相対パスへ固定する。
pub(super) fn entry_chunk_path(spec: &str, impl_name: &str) -> String {
    format!(
        "chunks/entries/{}/{}.js",
        encode_segment(spec),
        encode_segment(impl_name)
    )
}

/// file chunk は元の相対パス構造を維持し、差分を追いやすくする。
pub(super) fn file_chunk_path(spec: &str, impl_name: &str, path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let mut segments = normalized
        .split('/')
        .map(encode_segment)
        .collect::<Vec<_>>();
    let last = segments.pop().unwrap_or_else(|| "file".to_string());
    let file_name = format!("{last}.js");

    let mut chunk_path = format!(
        "chunks/files/{}/{}/",
        encode_segment(spec),
        encode_segment(impl_name)
    );
    if !segments.is_empty() {
        chunk_path.push_str(&segments.join("/"));
        chunk_path.push('/');
    }
    chunk_path.push_str(&file_name);
    chunk_path
}

fn encode_segment(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{entry_chunk_path, file_chunk_path};

    #[test]
    fn entry_chunk_path_is_stable() {
        assert_eq!(
            entry_chunk_path("tracey", "main"),
            "chunks/entries/tracey/main.js"
        );
    }

    #[test]
    fn file_chunk_path_keeps_directory_shape() {
        assert_eq!(
            file_chunk_path("tracey", "main", "src/lib.rs"),
            "chunks/files/tracey/main/src/lib.rs.js"
        );
    }
}
