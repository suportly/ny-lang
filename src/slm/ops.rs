//! src/slm/ops.rs

use super::tensor::Tensor;

pub fn rmsnorm(o: &mut Tensor, x: &Tensor, weight: &Tensor, size: usize) {
    let ss: f32 = x.data.iter().map(|&v| v * v).sum::<f32>() / size as f32;
    let ss = 1.0 / (ss + 1e-5).sqrt();
    for i in 0..size {
        o[i] = weight[i] * (ss * x[i]);
    }
}

pub fn matmul(out: &mut Tensor, x: &Tensor, w: &Tensor) {
    let n = x.shape()[0];
    let d = w.shape()[0];
    for i in 0..d {
        let mut val = 0.0;
        for j in 0..n {
            val += w.data[i * n + j] * x[j];
        }
        out[i] = val;
    }
}

pub fn rope(q: &mut Tensor, pos: usize, head_size: usize) {
    for i in (0..q.len()).step_by(2) {
        let head_dim = i % head_size;
        let freq = 1.0 / 10000.0f32.powf(head_dim as f32 / head_size as f32);
        let val = pos as f32 * freq;
        let fcr = val.cos();
        let fci = val.sin();
        let q0 = q[i];
        let q1 = q[i + 1];
        q[i] = q0 * fcr - q1 * fci;
        q[i+1] = q0 * fci + q1 * fcr;
    }
}

pub fn attention(out: &mut Tensor, q: &Tensor, k_cache: &Tensor, v_cache: &Tensor, pos: usize, n_head: usize, head_size: usize) {
    let dim = q.len();
    let mut att = vec![0.0; pos + 1];
    
    for h in 0..n_head {
        let q_head = &q.data[h*head_size..];
        let k_cache_head = &k_cache.data[h*head_size..];

        for t in 0..=pos {
            let mut score = 0.0;
            for i in 0..head_size {
                score += q_head[i] * k_cache_head[t * dim + i];
            }
            score /= (head_size as f32).sqrt();
            att[t] = score;
        }

        softmax(&mut att, pos + 1);

        let v_cache_head = &v_cache.data[h*head_size..];
        let out_head = &mut out.data[h*head_size..];

        for i in 0..head_size {
            let mut val = 0.0;
            for t in 0..=pos {
                val += att[t] * v_cache_head[t * dim + i];
            }
            out_head[i] = val;
        }
    }
}


pub fn silu(x: &mut Tensor, len: usize) {
    for i in 0..len {
        x[i] = x[i] * (1.0 / (1.0 + (-x[i]).exp()));
    }
}

pub fn add(a: &mut Tensor, b: &Tensor) {
    for (a_val, b_val) in a.data.iter_mut().zip(b.data.iter()) {
        *a_val += *b_val;
    }
}

pub fn mul(a: &mut Tensor, b: &Tensor) {
    for (a_val, b_val) in a.data.iter_mut().zip(b.data.iter()) {
        *a_val *= *b_val;
    }
}

pub fn softmax(x: &mut [f32], size: usize) {
    if size == 0 { return; }
    let max_val = x.iter().take(size).fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let mut sum = 0.0;
    for i in 0..size {
        x[i] = (x[i] - max_val).exp();
        sum += x[i];
    }
    for i in 0..size {
        x[i] /= sum;
    }
}


pub fn argmax(x: &Tensor) -> i32 {
    x.data
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(index, _)| index as i32)
        .unwrap_or(0)
}
