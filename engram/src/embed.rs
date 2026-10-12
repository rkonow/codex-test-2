//! Text embedders. Every vector is unit length, so Strata's dot product is
//! cosine similarity.
//!
//! * [`StaticEmbedder`]: a Model2Vec model (token lookup and mean). Fast
//!   enough to embed every hook payload inline.
//! * [`BertEmbedder`] (feature `bert`): BGE-small through candle.
//! * [`HashEmbedder`]: feature hashing of words, for tests without model files.

use std::path::{Path, PathBuf};

use crate::{err, Result};

pub trait Embedder: Send + Sync {
    fn name(&self) -> String;
    fn dim(&self) -> usize;
    /// Embeddings of documents.
    fn embed(&self, texts: &[&str]) -> Vec<Vec<f32>>;
    /// Embedding of a search query (some models prefix an instruction).
    fn embed_query(&self, text: &str) -> Vec<f32> {
        self.embed(&[text]).pop().unwrap()
    }
}

fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

/// Where models live: `$ENGRAM_MODELS`, else `$ENGRAM_HOME/models`.
pub fn models_dir() -> PathBuf {
    if let Ok(d) = std::env::var("ENGRAM_MODELS") {
        return PathBuf::from(d);
    }
    crate::paths::engram_home().join("models")
}

/// Opens an embedder from a spec: `static[:<dir>]`, `bert[:<dir>]` or `hash[:<dim>]`.
pub fn open(spec: &str) -> Result<Box<dyn Embedder>> {
    let (kind, arg) = spec.split_once(':').unwrap_or((spec, ""));
    match kind {
        "static" => {
            let dir = if arg.is_empty() { models_dir().join("potion-retrieval-32M") } else { PathBuf::from(arg) };
            Ok(Box::new(StaticEmbedder::open(&dir)?))
        }
        "hash" => Ok(Box::new(HashEmbedder { dim: arg.parse().unwrap_or(256) })),
        #[cfg(feature = "bert")]
        "bert" => {
            let dir = if arg.is_empty() { models_dir().join("bge-small-en-v1.5") } else { PathBuf::from(arg) };
            Ok(Box::new(BertEmbedder::open(&dir)?))
        }
        _ => Err(err(format!("unknown embedder {spec} (static, hash{})", if cfg!(feature = "bert") { ", bert" } else { "" }))),
    }
}

// ---------------------------------------------------------------- static

/// Model2Vec: the embedding of a text is the mean of its tokens' rows.
pub struct StaticEmbedder {
    tokenizer: tokenizers::Tokenizer,
    table: Vec<f32>,
    dim: usize,
    unk: Option<u32>,
    name: String,
    max_tokens: usize,
}

impl StaticEmbedder {
    pub fn open(dir: &Path) -> Result<StaticEmbedder> {
        let tokenizer = tokenizers::Tokenizer::from_file(dir.join("tokenizer.json"))
            .map_err(|e| err(format!("{}: {e} (run `engram fetch-model`)", dir.join("tokenizer.json").display())))?;
        let (table, rows, dim) = read_safetensors_matrix(&dir.join("model.safetensors"), "embeddings")?;
        if rows < tokenizer.get_vocab_size(true) {
            return Err(err(format!("embedding table has {rows} rows, tokenizer {} tokens", tokenizer.get_vocab_size(true))));
        }
        let unk = tokenizer.token_to_id("[UNK]");
        let name = format!("static:{}", dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        Ok(StaticEmbedder { tokenizer, table, dim, unk, name, max_tokens: 512 })
    }

    fn embed_ids(&self, ids: &[u32]) -> Vec<f32> {
        let mut v = vec![0f32; self.dim];
        let mut n = 0usize;
        for &id in ids.iter().filter(|&&id| Some(id) != self.unk).take(self.max_tokens) {
            let row = &self.table[id as usize * self.dim..(id as usize + 1) * self.dim];
            v.iter_mut().zip(row).for_each(|(a, b)| *a += b);
            n += 1;
        }
        if n > 0 {
            v.iter_mut().for_each(|x| *x /= n as f32);
            normalize(&mut v);
        }
        v
    }
}

impl Embedder for StaticEmbedder {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed(&self, texts: &[&str]) -> Vec<Vec<f32>> {
        texts
            .iter()
            .map(|t| match self.tokenizer.encode(*t, false) {
                Ok(e) => self.embed_ids(e.get_ids()),
                Err(_) => vec![0f32; self.dim],
            })
            .collect()
    }
}

/// Reads one F32 tensor of a safetensors file as a row-major matrix.
fn read_safetensors_matrix(path: &Path, tensor: &str) -> Result<(Vec<f32>, usize, usize)> {
    let bytes = std::fs::read(path).map_err(|e| err(format!("{}: {e} (run `engram fetch-model`)", path.display())))?;
    if bytes.len() < 8 {
        return Err(err(format!("{}: not a safetensors file", path.display())));
    }
    let hlen = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    let header: serde_json::Value = serde_json::from_slice(&bytes[8..8 + hlen]).map_err(|e| err(format!("{}: {e}", path.display())))?;
    let t = header.get(tensor).ok_or_else(|| err(format!("{}: no tensor {tensor}", path.display())))?;
    if t["dtype"] != "F32" {
        return Err(err(format!("{tensor}: dtype {} (need F32)", t["dtype"])));
    }
    let shape: Vec<usize> = t["shape"].as_array().into_iter().flatten().filter_map(|x| x.as_u64()).map(|x| x as usize).collect();
    let offs: Vec<usize> = t["data_offsets"].as_array().into_iter().flatten().filter_map(|x| x.as_u64()).map(|x| x as usize).collect();
    let [rows, dim] = shape[..] else { return Err(err(format!("{tensor}: shape {shape:?}"))) };
    let data = &bytes[8 + hlen + offs[0]..8 + hlen + offs[1]];
    let table = data.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>();
    if table.len() != rows * dim {
        return Err(err(format!("{tensor}: {} values for shape {rows}x{dim}", table.len())));
    }
    Ok((table, rows, dim))
}

// ---------------------------------------------------------------- hash

/// Lowercased words hashed into `dim` buckets (signed), for tests.
pub struct HashEmbedder {
    pub dim: usize,
}

impl Embedder for HashEmbedder {
    fn name(&self) -> String {
        format!("hash:{}", self.dim)
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed(&self, texts: &[&str]) -> Vec<Vec<f32>> {
        texts
            .iter()
            .map(|t| {
                let mut v = vec![0f32; self.dim];
                for w in t.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
                    let h = fnv(&w.to_lowercase());
                    v[(h % self.dim as u64) as usize] += if h >> 63 == 0 { 1.0 } else { -1.0 };
                }
                normalize(&mut v);
                v
            })
            .collect()
    }
}

pub fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ---------------------------------------------------------------- bert

#[cfg(feature = "bert")]
pub struct BertEmbedder {
    model: candle_transformers::models::bert::BertModel,
    tokenizer: tokenizers::Tokenizer,
    dim: usize,
    name: String,
}

#[cfg(feature = "bert")]
impl BertEmbedder {
    const QUERY_PREFIX: &'static str = "Represent this sentence for searching relevant passages: ";

    pub fn open(dir: &Path) -> Result<BertEmbedder> {
        use candle_nn::VarBuilder;
        use candle_transformers::models::bert::{BertModel, Config};
        let e = |x: String| err(format!("{}: {x}", dir.display()));
        let cfg: Config = serde_json::from_str(&std::fs::read_to_string(dir.join("config.json")).map_err(|x| e(x.to_string()))?).map_err(|x| e(x.to_string()))?;
        let dim = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(dir.join("config.json")).unwrap())
            .ok()
            .and_then(|v| v["hidden_size"].as_u64())
            .unwrap_or(384) as usize;
        let mut tokenizer = tokenizers::Tokenizer::from_file(dir.join("tokenizer.json")).map_err(|x| e(x.to_string()))?;
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams { max_length: 512, ..Default::default() }))
            .map_err(|x| e(x.to_string()))?;
        tokenizer.with_padding(Some(tokenizers::PaddingParams::default()));
        let dev = candle_core::Device::Cpu;
        // SAFETY: the weights file is not modified while mapped.
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[dir.join("model.safetensors")], candle_core::DType::F32, &dev) }.map_err(|x| e(x.to_string()))?;
        let model = BertModel::load(vb, &cfg).map_err(|x| e(x.to_string()))?;
        let name = format!("bert:{}", dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        Ok(BertEmbedder { model, tokenizer, dim, name })
    }

    fn run(&self, texts: &[String]) -> Vec<Vec<f32>> {
        use candle_core::Tensor;
        let attempt = || -> std::result::Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
            let enc = self.tokenizer.encode_batch(texts.to_vec(), true).map_err(|e| e.to_string())?;
            let ids: Vec<Vec<u32>> = enc.iter().map(|e| e.get_ids().to_vec()).collect();
            let mask: Vec<Vec<u32>> = enc.iter().map(|e| e.get_attention_mask().to_vec()).collect();
            let dev = candle_core::Device::Cpu;
            let ids = Tensor::new(ids, &dev)?;
            let mask = Tensor::new(mask, &dev)?;
            let types = ids.zeros_like()?;
            let out = self.model.forward(&ids, &types, Some(&mask))?;
            let cls = out.narrow(1, 0, 1)?.squeeze(1)?;
            let mut v = cls.to_vec2::<f32>()?;
            v.iter_mut().for_each(|x| normalize(x));
            Ok(v)
        };
        attempt().unwrap_or_else(|_| texts.iter().map(|_| vec![0f32; self.dim]).collect())
    }
}

#[cfg(feature = "bert")]
impl Embedder for BertEmbedder {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed(&self, texts: &[&str]) -> Vec<Vec<f32>> {
        // Batches of similar length waste less on padding.
        let mut order: Vec<usize> = (0..texts.len()).collect();
        order.sort_by_key(|&i| texts[i].len());
        let mut out = vec![Vec::new(); texts.len()];
        for chunk in order.chunks(16) {
            let batch: Vec<String> = chunk.iter().map(|&i| texts[i].to_string()).collect();
            for (&i, v) in chunk.iter().zip(self.run(&batch)) {
                out[i] = v;
            }
        }
        out
    }
    fn embed_query(&self, text: &str) -> Vec<f32> {
        self.run(&[format!("{}{text}", Self::QUERY_PREFIX)]).pop().unwrap()
    }
}
