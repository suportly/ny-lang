//! src/slm/model.rs

use super::tensor::Tensor;

pub struct Config {
    pub dim: usize,
    pub hidden_dim: usize,
    pub n_layer: usize,
    pub n_head: usize,
    pub n_kv_head: usize,
    pub vocab_size: usize,
    pub seq_len: usize,
    pub head_size: usize,
}

impl Config {
    pub fn new() -> Self {
        let dim = 288;
        let n_layer = 6;
        let n_head = 6;
        Self {
            dim,
            hidden_dim: 768,
            n_layer,
            n_head,
            n_kv_head: n_head,
            vocab_size: 32000,
            seq_len: 256,
            head_size: dim / n_head,
        }
    }
}

pub struct Weights {
    pub token_embedding_table: Tensor,
    pub rms_att_weight: Vec<Tensor>,
    pub wq: Vec<Tensor>,
    pub wk: Vec<Tensor>,
    pub wv: Vec<Tensor>,
    pub wo: Vec<Tensor>,
    pub rms_ffn_weight: Vec<Tensor>,
    pub w1: Vec<Tensor>,
    pub w2: Vec<Tensor>,
    pub w3: Vec<Tensor>,
    pub rms_final_weight: Tensor,
}

impl Weights {
    pub fn new(config: &Config) -> Self {
        let dim = config.dim;
        let hidden_dim = config.hidden_dim;
        let n_layer = config.n_layer;
        let vocab_size = config.vocab_size;

        Self {
            token_embedding_table: Tensor::zeros(vec![vocab_size, dim]),
            rms_att_weight: vec![Tensor::zeros(vec![dim]); n_layer],
            wq: vec![Tensor::zeros(vec![dim, dim]); n_layer],
            wk: vec![Tensor::zeros(vec![dim, dim]); n_layer],
            wv: vec![Tensor::zeros(vec![dim, dim]); n_layer],
            wo: vec![Tensor::zeros(vec![dim, dim]); n_layer],
            rms_ffn_weight: vec![Tensor::zeros(vec![dim]); n_layer],
            w1: vec![Tensor::zeros(vec![hidden_dim, dim]); n_layer],
            w2: vec![Tensor::zeros(vec![dim, hidden_dim]); n_layer],
            w3: vec![Tensor::zeros(vec![hidden_dim, dim]); n_layer],
            rms_final_weight: Tensor::zeros(vec![dim]),
        }
    }
}

pub struct RunState {
    pub x: Tensor,
    pub xb: Tensor,
    pub xb2: Tensor,
    pub hb: Tensor,
    pub hb2: Tensor,
    pub q: Tensor,
    pub k: Tensor,
    pub v: Tensor,
    pub att: Tensor,
    pub logits: Tensor,
    pub key_cache: Vec<Tensor>,
    pub value_cache: Vec<Tensor>,
}

impl RunState {
    pub fn new(config: &Config) -> Self {
        let dim = config.dim;
        let hidden_dim = config.hidden_dim;
        let n_layer = config.n_layer;
        let seq_len = config.seq_len;
        let vocab_size = config.vocab_size;

        Self {
            x: Tensor::zeros(vec![dim]),
            xb: Tensor::zeros(vec![dim]),
            xb2: Tensor::zeros(vec![dim]),
            hb: Tensor::zeros(vec![hidden_dim]),
            hb2: Tensor::zeros(vec![hidden_dim]),
            q: Tensor::zeros(vec![dim]),
            k: Tensor::zeros(vec![dim]),
            v: Tensor::zeros(vec![dim]),
            att: Tensor::zeros(vec![dim]),
            logits: Tensor::zeros(vec![vocab_size]),
            key_cache: vec![Tensor::zeros(vec![seq_len, dim]); n_layer],
            value_cache: vec![Tensor::zeros(vec![seq_len, dim]); n_layer],
        }
    }
}


pub struct SLModel {
    pub config: Config,
    pub weights: Weights,
}

impl SLModel {
    pub fn new() -> Self {
        let config = Config::new();
        let weights = Weights::new(&config);
        Self { config, weights }
    }
}
