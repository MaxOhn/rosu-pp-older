use std::{
    iter::Copied,
    slice::{Iter, IterMut},
};

#[derive(Clone)]
pub struct StrainsVec {
    inner: Vec<f64>,
}

impl StrainsVec {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn push(&mut self, value: f64) {
        self.inner.push(value);
    }

    pub fn sort_desc(&mut self) {
        self.inner.sort_by(|a, b| b.total_cmp(a));
    }

    pub fn retain_non_zero(&mut self) {
        self.inner.retain(|&a| a > 0.0);
    }

    pub fn retain_non_zero_and_sort(&mut self) {
        self.retain_non_zero();
        self.sort_desc();
    }

    pub fn non_zero_iter(&self) -> Copied<Iter<'_, f64>> {
        self.inner.iter().copied()
    }

    pub fn sorted_non_zero_iter(&mut self) -> Copied<Iter<'_, f64>> {
        self.retain_non_zero_and_sort();

        self.non_zero_iter()
    }

    pub fn sorted_non_zero_iter_mut(&mut self) -> IterMut<'_, f64> {
        self.retain_non_zero_and_sort();

        self.inner.iter_mut()
    }

    pub fn sum(&self) -> f64 {
        self.inner.iter().copied().sum()
    }

    pub fn iter(&self) -> Copied<Iter<'_, f64>> {
        self.inner.iter().copied()
    }

    pub fn into_vec(self) -> Vec<f64> {
        self.inner
    }
}
