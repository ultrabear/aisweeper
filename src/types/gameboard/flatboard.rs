use std::iter::repeat;
use std::ops::{Index, IndexMut};

#[derive(Debug, PartialEq)]
pub struct FlatBoard<T> {
	dim_1: usize,
	dim_2: usize,
	data: Box<[T]>,
}

pub type Row<T> = [T];

pub trait IterBacking<'a, T: 'a, I: Iterator<Item = &'a T>> {
	fn iter_backing(&'a self) -> I;
}

pub trait IterBackingMut<'a, T: 'a, I: Iterator<Item = &'a mut T>> {
	fn iter_backing_mut(&'a mut self) -> I;
}

impl<T: Clone> FlatBoard<T> {
	/// generates a new 2d array
	pub fn new(dim_1: usize, dim_2: usize, default: T) -> Self {
		let array_len = dim_1
			.checked_mul(dim_2)
			.expect("array length overflowed usize");

		Self {
			dim_1,
			dim_2,
			data: repeat(default).take(array_len).collect(),
		}
	}
}

impl<T> FlatBoard<T> {
	/// returns dimensions of flatboard
	#[inline(always)]
	pub fn dimensions(&self) -> (usize, usize) {
		(self.dim_1, self.dim_2)
	}

	/// returns dim_1 as len
	#[inline(always)]
	pub fn len(&self) -> usize {
		self.dim_1
	}

	#[inline(always)]
	pub fn get(&self, idx: usize) -> Option<&Row<T>> {
		if self.dim_1 <= idx {
			None
		} else {
			let idxstart = idx * self.dim_2;

			Some(&self.data[idxstart..idxstart + self.dim_2])
		}
	}

	#[inline(always)]
	pub fn get_mut(&mut self, idx: usize) -> Option<&mut Row<T>> {
		if self.dim_1 <= idx {
			None
		} else {
			let idxstart = idx * self.dim_2;

			Some(&mut self.data[idxstart..idxstart + self.dim_2])
		}
	}

	#[inline(always)]
	pub fn iter(&self) -> impl Iterator<Item = &[T]> {
		self.data.chunks(self.dim_2)
	}

	#[inline(always)]
	pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut [T]> {
		self.data.chunks_mut(self.dim_2)
	}
}

use std::slice::{Iter, IterMut};

impl<'a, T: 'a> IterBacking<'a, T, Iter<'a, T>> for FlatBoard<T> {
	fn iter_backing(&'a self) -> Iter<'a, T> {
		self.data.iter()
	}
}

impl<'a, T: 'a> IterBackingMut<'a, T, IterMut<'a, T>> for FlatBoard<T> {
	fn iter_backing_mut(&'a mut self) -> IterMut<'a, T> {
		self.data.iter_mut()
	}
}

impl<T> Index<usize> for FlatBoard<T> {
	type Output = Row<T>;

	#[inline(always)]
	fn index(&self, index: usize) -> &Self::Output {
		match self.get(index) {
			None => panic!(
				"Index {index} out of bounds (FlatBoard has a length of {})",
				self.dim_1
			),
			Some(v) => v,
		}
	}
}

impl<T> IndexMut<usize> for FlatBoard<T> {
	#[inline(always)]
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		// hack around get_mut mutable borrow invalidating immutable borrows to self
		let dim_1 = self.dim_1;

		match self.get_mut(index) {
			None => panic!(
				"Index {index} out of bounds (FlatBoard has a length of {})",
				dim_1
			),
			Some(v) => v,
		}
	}
}
