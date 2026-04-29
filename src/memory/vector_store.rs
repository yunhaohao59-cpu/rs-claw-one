pub struct VectorStore {
    index_path: std::path::PathBuf,
}

impl VectorStore {
    pub fn new(path: std::path::PathBuf) -> anyhow::Result<Self> {
        Ok(Self { index_path: path })
    }

    pub fn insert(&self, _id: &str, _vector: &[f32]) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn search(
        &self,
        _query: &[f32],
        _top_k: usize,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        Ok(Vec::new())
    }
}
