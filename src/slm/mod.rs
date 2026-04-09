//! src/slm/mod.rs

pub mod tensor;
pub mod ops;
pub mod model;

use model::{SLModel, Config, RunState};

pub fn forward(model: &mut SLModel, tokenizer: &Tokenizer, prompt: &str) -> String {
    let tokens = tokenizer.encode(prompt);
    let mut state = RunState::new(&model.config);

    for (pos, token) in tokens.iter().enumerate() {
        forward_step(model, &mut state, *token, pos);
    }

    let next_token = sample(model, &mut state);
    tokenizer.decode(&[next_token])
}

fn forward_step(model: &mut SLModel, state: &mut RunState, token: i32, pos: usize) {
    let config = &model.config;
    let weights = &model.weights;

    let x = &mut state.x;
    let xb = &mut state.xb;
    let xb2 = &mut state.xb2;
    let hb = &mut state.hb;
    let hb2 = &mut state.hb2;
    let q = &mut state.q;
    let k = &mut state.k;
    let v = &mut state.v;
    let att = &mut state.att;
    let logits = &mut state.logits;
    let key_cache = &mut state.key_cache;
    let value_cache = &mut state.value_cache;

    let token_embedding = weights.token_embedding_table.row(token as usize);
    x.copy_from_slice(&token_embedding);

    for l in 0..config.n_layer {
        ops::rmsnorm(xb, x, &weights.rms_att_weight[l], config.dim);

        ops::matmul(q, xb, &weights.wq[l]);
        ops::matmul(k, xb, &weights.wk[l]);
        ops::matmul(v, xb, &weights.wv[l]);

        ops::rope(q, pos, config.head_size);
        ops::rope(k, pos, config.head_size);

        key_cache[l][pos].copy_from_slice(k);
        value_cache[l][pos].copy_from_slice(v);

        ops::attention(att, q, &key_cache[l], &value_cache[l], pos, config.n_head, config.head_size);

        ops::matmul(xb2, att, &weights.wo[l]);

        ops::add(x, xb2);

        ops::rmsnorm(hb, x, &weights.rms_ffn_weight[l], config.dim);

        ops::matmul(hb2, hb, &weights.w1[l]);
        ops::silu(hb2, config.hidden_dim);
        ops::matmul(hb, hb2, &weights.w3[l]);
        ops::mul(hb, &weights.w2[l]);

        ops::add(x, hb);
    }

    ops::rmsnorm(x, x, &weights.rms_final_weight, config.dim);
    ops::matmul(logits, x, &weights.token_embedding_table);
}

fn sample(model: &SLModel, state: &mut RunState) -> i32 {
    ops::softmax(&mut state.logits, 1);
    ops::argmax(&state.logits)
}

pub struct Tokenizer {
    vocab: Vec<String>,
}

impl Tokenizer {
    pub fn new(vocab: Vec<String>) -> Self {
        Self { vocab }
    }

    pub fn encode(&self, text: &str) -> Vec<i32> {
        // Simplified tokenizer
        text.chars().map(|c| c as i32).collect()
    }

    pub fn decode(&self, tokens: &[i32]) -> String {
        tokens.iter().map(|&t| std::char::from_u32(t as u32).unwrap_or('�')).collect()
    }
}
