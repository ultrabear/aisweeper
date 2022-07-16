use std::iter::repeat;
use std::ops::{Index, IndexMut};

#[derive(Debug, PartialEq)]
pub struct FlatBoard<T> {
	dim_1: usize,
	dim_2: usize,
	data: Box<[T]>,
}

// manually implementing circumvents T being default
impl<T> Default for FlatBoard<T> {
	fn default() -> Self {
		Self {
			dim_1: 0,
			dim_2: 0,
			data: Box::new([]),
		}
	}
}

pub type Row<T> = [T];

pub trait IterBacking<'a, T: 'a, I: Iterator<Item = &'a T>> {
	fn iter_backing(&'a self) -> I;
}

pub trait IterBackingMut<'a, T: 'a, I: Iterator<Item = &'a mut T>> {
	fn iter_backing_mut(&'a mut self) -> I;
}

use std::iter;
use std::slice;

// vector implementations
type MutVecBacking<'a, T> = iter::Flatten<slice::IterMut<'a, Vec<T>>>;
impl<'a, T> IterBackingMut<'a, T, MutVecBacking<'a, T>> for Vec<Vec<T>> {
	fn iter_backing_mut(&'a mut self) -> MutVecBacking<'a, T> {
		self.iter_mut().flatten()
	}
}

type VecBacking<'a, T> = iter::Flatten<slice::Iter<'a, Vec<T>>>;
impl<'a, T> IterBacking<'a, T, VecBacking<'a, T>> for Vec<Vec<T>> {
	fn iter_backing(&'a self) -> VecBacking<'a, T> {
		self.iter().flatten()
	}
}

// new constructor
impl<T: Clone> FlatBoard<T> {
	/// generates a new 2d array using default as fill in
	pub fn new(dim_1: usize, dim_2: usize, default: T) -> Self {
		let array_len = Self::array_length(dim_1, dim_2);

		Self {
			dim_1,
			dim_2,
			data: repeat(default).take(array_len).collect(),
		}
	}
}

impl<T: Default> FlatBoard<T> {
	/// generates a new 2d array using T's default method as fill in
	pub fn new_default(dim_1: usize, dim_2: usize) -> Self {
		let array_len = Self::array_length(dim_1, dim_2);

		Self {
			dim_1,
			dim_2,
			data: repeat(()).map(|_| T::default()).take(array_len).collect(),
		}
	}
}

impl<T> FlatBoard<T> {
	/// helper method to return an arrays length from its 2 dimensions or panic on overflow
	fn array_length(dim_1: usize, dim_2: usize) -> usize {
		dim_1
			.checked_mul(dim_2)
			.expect("array length overflowed usize")
	}

	/// returns dimensions of flatboard
	#[inline]
	pub const fn dimensions(&self) -> (usize, usize) {
		(self.dim_1, self.dim_2)
	}

	/// returns dim_1 as len
	#[inline]
	pub const fn len(&self) -> usize {
		self.dim_1
	}

	/// gets a reference to a [`Row`] of T from the board at the given index, or returns [`None`] on out of bounds
	#[inline]
	pub fn get(&self, idx: usize) -> Option<&Row<T>> {
		if self.dim_1 <= idx {
			None
		} else {
			let idxstart = idx * self.dim_2;

			Some(&self.data[idxstart..idxstart + self.dim_2])
		}
	}

	/// gets a mutable reference to a [`Row`] of T from the board at the given index, or returns [`None`] on out of bounds
	#[inline]
	pub fn get_mut(&mut self, idx: usize) -> Option<&mut Row<T>> {
		if self.dim_1 <= idx {
			None
		} else {
			let idxstart = idx * self.dim_2;

			Some(&mut self.data[idxstart..idxstart + self.dim_2])
		}
	}

	/// iterates over the board, returning each [`Row`] in sequence
	#[inline]
	pub fn iter(&self) -> impl Iterator<Item = &Row<T>> {
		self.data.chunks(self.dim_2)
	}

	/// iterates over the board mutably, returning each [`Row`] in sequence
	#[inline]
	pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Row<T>> {
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

	#[inline]
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
	#[inline]
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
