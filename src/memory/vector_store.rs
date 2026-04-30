use std::path::PathBuf;

pub struct VectorStore {
    index_path: PathBuf,
}

fn trigram_embedding(text: &str, dim: usize) -> Vec<f32> {
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut vector = vec![0.0f32; dim];

    for window in chars.windows(3) {
        let seed: u64 = window.iter().fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(*c as u64));
        let idx = (seed % dim as u64) as usize;
        vector[idx] += 1.0;
    }

    let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vector {
            *v /= norm;
        }
    }

    vector
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-8 || nb < 1e-8 { return 0.0; }
    dot / (na * nb)
}

impl VectorStore {
    pub fn new(index_path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { index_path })
    }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        trigram_embedding(text, 256)
    }

    pub fn insert(&self, id: &str, text: &str) -> anyhow::Result<()> {
        let vector = self.embed(text);
        let path = self.index_path.join(format!("{}.vec", id));
        let bytes: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
        std::fs::create_dir_all(&self.index_path)?;
        std::fs::write(&path, bytes)?;
        Ok(())
    }

    pub fn search(&self, query_text: &str, top_k: usize) -> anyhow::Result<Vec<(String, f32)>> {
        let query_vec = self.embed(query_text);
        let mut results: Vec<(String, f32)> = Vec::new();

        let entries = std::fs::read_dir(&self.index_path)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "vec") {
                let id = path.file_stem().unwrap().to_string_lossy().to_string();
                let bytes = std::fs::read(&path)?;
                let mut vec = Vec::with_capacity(bytes.len() / 4);
                for chunk in bytes.chunks_exact(4) {
                    vec.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                }
                if vec.len() == query_vec.len() {
                    let sim = cosine_similarity(&query_vec, &vec);
                    results.push((id, sim));
                }
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        Ok(results)
    }

    pub fn delete(&self, id: &str) -> anyhow::Result<()> {
        let path = self.index_path.join(format!("{}.vec", id));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
