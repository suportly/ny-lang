//! src/slm/tensor.rs

use std::ops::{Index, IndexMut};

#[derive(Debug, Clone)]
pub struct Tensor {
    pub data: Vec<f32>,
    shape: Vec<usize>,
}

impl Tensor {
    pub fn new(data: Vec<f32>, shape: Vec<usize>) -> Self {
        assert_eq!(data.len(), shape.iter().product::<usize>());
        Self { data, shape }
    }

    pub fn zeros(shape: Vec<usize>) -> Self {
        let size = shape.iter().product::<usize>();
        Self {
            data: vec![0.0; size],
            shape,
        }
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn row(&self, index: usize) -> &[f32] {
        assert_eq!(self.shape.len(), 2);
        let cols = self.shape[1];
        let start = index * cols;
        &self.data[start..start + cols]
    }
    
    pub fn copy_from_slice(&mut self, slice: &[f32]) {
        self.data.copy_from_slice(slice);
    }
}

impl Index<usize> for Tensor {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl IndexMut<usize> for Tensor {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}
