use bytemuck::Pod;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Matrix2<T> {
    pub data: Vec<T>,
    pub rows: usize,
    pub cols: usize,
}

impl<T: Clone> Matrix2<T> {
    pub fn new(rows: usize, cols: usize, init: T) -> Self {
        Self {
            data: vec![init; rows * cols],
            rows,
            cols,
        }
    }
    pub fn get(&self, row: usize, col: usize) -> &T {
        &self.data[row * self.cols + col]
    }
    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut T {
        &mut self.data[row * self.cols + col]
    }
}

// Implement PartialEq, Eq and Hash using a byte-wise comparison.
impl<T: Pod> PartialEq for Matrix2<T> {
    fn eq(&self, other: &Self) -> bool {
        self.rows == other.rows
            && self.cols == other.cols
            && bytemuck::cast_slice::<T, u8>(&self.data)
                == bytemuck::cast_slice::<T, u8>(&other.data)
    }
}

impl<T: Pod> Eq for Matrix2<T> {}

impl<T: Pod> Hash for Matrix2<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rows.hash(state);
        self.cols.hash(state);
        bytemuck::cast_slice::<T, u8>(&self.data).hash(state);
    }
}

/// A simple 3D matrix that wraps a flat Vec.
#[derive(Debug, Clone)]
pub struct Matrix3<T> {
    data: Vec<T>,
    dim2: usize,
    dim3: usize,
}

impl<T: Clone> Matrix3<T> {
    pub fn new(dim1: usize, dim2: usize, dim3: usize, init: T) -> Self {
        Self {
            data: vec![init; dim1 * dim2 * dim3],
            dim2,
            dim3,
        }
    }
    pub fn get(&self, i: usize, j: usize, k: usize) -> &T {
        &self.data[i * self.dim2 * self.dim3 + j * self.dim3 + k]
    }
    pub fn get_mut(&mut self, i: usize, j: usize, k: usize) -> &mut T {
        &mut self.data[i * self.dim2 * self.dim3 + j * self.dim3 + k]
    }
}
