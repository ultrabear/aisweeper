use std::fmt;
use std::iter::repeat;

use rand::prelude::*;

use super::tiles;
use tiles::{BoardTile, Tile, Visibility, VisibleTile};

use super::errors::{assert_not_bomb, NewBoardError, UndoError, UnopenableError};

mod flatboard;
pub use flatboard::{FlatBoard, IterBacking, IterBackingMut};

#[derive(Debug)]
pub struct GameBoard {
	pub(self) bombs: u32,
	// board is indexed as y/x but the api uses x/y
	pub(self) board: FlatBoard<BoardTile>,
}

/*fn gen_2d_array<T: Clone>(x: usize, y: usize, insert: T) -> Vec<Vec<T>> {

	repeat(repeat(insert).take(y).collect()).take(x).collect()
}*/

fn gen_2d_array<T: Clone>(x: usize, y: usize, insert: T) -> FlatBoard<T> {
	FlatBoard::new(x, y, insert)
}

/// an event that gives full detail to undo the action in an efficient manner, at the cost of memory use.
pub enum GameBoardEvent {
	/// a opening of a set of cells, represented by an array of x/y coordinates
	OpenCell(Box<[(u16, u16)]>),
	/// a flag/unflag of a cell
	ToggleFlagCell(u16, u16),
}

impl From<Vec<(u16, u16)>> for GameBoardEvent {
	fn from(v: Vec<(u16, u16)>) -> Self {
		Self::OpenCell(v.into())
	}
}

impl GameBoardEvent {
	pub fn flag_cell(x: u16, y: u16) -> Self {
		Self::ToggleFlagCell(x, y)
	}
}

impl<'a, T: 'a> IterBackingMut<'a, T, std::iter::Flatten<std::slice::IterMut<'a, Vec<T>>>>
	for Vec<Vec<T>>
{
	fn iter_backing_mut(&'a mut self) -> std::iter::Flatten<std::slice::IterMut<'a, Vec<T>>> {
		self.iter_mut().flatten()
	}
}

impl<'a, T: 'a> IterBacking<'a, T, std::iter::Flatten<std::slice::Iter<'a, Vec<T>>>>
	for Vec<Vec<T>>
{
	fn iter_backing(&'a self) -> std::iter::Flatten<std::slice::Iter<'a, Vec<T>>> {
		self.iter().flatten()
	}
}

fn widening_mul(a: u16, b: u16) -> u32 {
	u32::from(a) * u32::from(b)
}

fn widen_xy(x: u16, y: u16) -> (usize, usize) {
	(x.into(), y.into())
}

/// basic utils
impl GameBoard {
	pub fn bomb_count(&self) -> u32 {
		self.bombs
	}

	/// returns bomb density as a percentage in the range \[0,1\]
	pub fn bomb_density(&self) -> f64 {
		f64::from(self.bomb_count()) / f64::from(self.area())
	}

	/// returns the x/y dimensions of a game board
	pub fn dimensions(&self) -> (u16, u16) {
		(
			self.board.dimensions().1.try_into().unwrap(),
			self.board.len().try_into().unwrap(),
		)
	}

	/// returns the computed x*y area of a game board
	pub fn area(&self) -> u32 {
		let (x, y) = self.dimensions();
		widening_mul(x, y)
	}

	/// builds every point that is accessible in a 3x3 grid around a specified point
	fn normalize_around_3x3(&self, orig_x: u16, orig_y: u16) -> Vec<(usize, usize)> {
		// SAFETY: Converted back to usize before use, only used as indexing so negatives will overflow
		let startx = (orig_x as isize) - 1;
		let endx = (orig_x as isize) + 1;
		let starty = (orig_y as isize) - 1;
		let endy = (orig_y as isize) + 1;

		let mut arr = Vec::with_capacity(8);

		for y in (starty..=endy).map(|i| i as usize) {
			for x in (startx..=endx).map(|i| i as usize) {
				if (|| self.board.get(y)?.get(x))().is_some() {
					// disallow origin being selected
					if !((usize::from(orig_x) == x) && (usize::from(orig_y) == y)) {
						arr.push((x, y))
					}
				}
			}
		}

		arr
	}

	/// validates that bomb counts and size counts do not exceed hard coded limits for sanity
	fn validate_size_constraints(x: u16, y: u16, bombs: u32) -> Result<(), NewBoardError> {
		if x > 10_000 || y > 10_000 || bombs > 100_000_000 {
			Err(NewBoardError::SizeConstraintOverflow)
		} else {
			Ok(())
		}
	}

	fn tile_or_unopenable(&self, x: u16, y: u16) -> Result<BoardTile, UnopenableError> {
		self.get(x, y).ok_or(UnopenableError::OutOfBounds)
	}

	fn get(&self, x: u16, y: u16) -> Option<BoardTile> {
		let (x, y) = widen_xy(x, y);
		self.board.get(y)?.get(x).copied()
	}

	fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut BoardTile> {
		let (x, y) = widen_xy(x, y);

		self.board.get_mut(y)?.get_mut(x)
	}
}

// game logic
impl GameBoard {
	/// computes the amount of bombs around a tile by reading all 8 tiles around a tile
	fn computed_bombs_around_tile(&self, unsigned_x: u16, unsigned_y: u16) -> u8 {
		let readable = self.normalize_around_3x3(unsigned_x, unsigned_y);

		let mut bombcount = 0u8;

		for (xoff, yoff) in readable.into_iter() {
			if self.board[yoff][xoff].tile.is_bomb() {
				bombcount += 1
			}
		}

		bombcount
	}

	/// implants a vector of bombs into the board
	fn _populate_implant(&mut self, arr: Vec<bool>) {
		let mut it = arr.into_iter();

		for t in self.board.iter_backing_mut() {
			t.tile = if it.next().expect("Bomb vector was not correctly sized") {
				Tile::Bomb
			} else {
				Tile::Zero
			};
		}

		// required to borrow computed_bombs_around_tile immutably
		for y in 0..self.board.len() {
			for x in 0..self.board[y].len() {
				if !(self.board[y][x].tile == Tile::Bomb) {
					self.board[y][x].tile = self
						.computed_bombs_around_tile(x as u16, y as u16)
						.try_into()
						.expect("More than 8 bombs surrounding tile (impossible invariant)");
				}
			}
		}
	}

	/// populates a minesweeper board with bombs and computes tiles around it
	fn populate(&mut self) {
		let mut rng = rand::thread_rng();

		let mut arr: Vec<bool> = repeat(true)
			.take(self.bombs.try_into().expect("bomb count overflowed usize"))
			.chain(repeat(false))
			.take(self.area().try_into().expect("area overflowed usize"))
			.collect();

		arr.shuffle(&mut rng);
		arr.shuffle(&mut rng);

		self._populate_implant(arr);
	}

	/// populates a board with bombs without bombs around a certain xy coordinate in a 3x3 grid
	fn populate_without(&mut self, x: u16, y: u16) -> Result<(), NewBoardError> {
		let mut valid = self.normalize_around_3x3(x, y);
		// include self in valid
		valid.push((x.into(), y.into()));

		if (self.area() - self.bombs) < valid.len() as u32 {
			return Err(NewBoardError::BombOverflow);
		}

		let mut rng = rand::thread_rng();

		// SAFETY: panics are impossible on 64 bit machines due to bombcount and area being u32
		// 32 bit machines might overflow isize constraints, but at that point there is no memory left
		let mut arr: Vec<bool> = repeat(true)
			.take(
				self.bomb_count()
					.try_into()
					.expect("bomb count overflowed usize"),
			)
			.chain(repeat(false))
			.take(self.area().try_into().expect("area overflowed usize"))
			.collect();

		arr.shuffle(&mut rng);

		// flattens a [y][x] indexed flat array into its true index
		let flatten = |x, y| ((y * usize::from(self.dimensions().0)) + x);

		let mut reroute = true;

		// keep looping until no bombs were rerouted
		while reroute {
			reroute = false;

			for (x, y) in valid.iter().copied() {
				if arr[flatten(x, y)] {
					reroute = true;

					arr.swap(flatten(x, y), rng.gen_range(0..self.area() as usize));
				}
			}
		}

		self._populate_implant(arr);
		Ok(())
	}

	/// generates a blank board without adding bombs to it, but stores bomb count
	fn blank_board(x: u16, y: u16, bombs: u32) -> Result<Self, NewBoardError> {
		Self::validate_size_constraints(x, y, bombs)?;

		let area: u32 = u32::from(x) * u32::from(y);
		if area < bombs {
			return Err(NewBoardError::BombOverflow);
		}

		if x == 0 || y == 0 {
			return Err(NewBoardError::ZeroDimension);
		}

		let gb = Self {
			bombs,
			board: gen_2d_array(
				y.into(),
				x.into(),
				BoardTile {
					tile: Tile::Zero,
					visible: Visibility::NotVisible,
				},
			),
		};

		Ok(gb)
	}

	/// generates a new board with a given clear zone where no bombs will be guaranteed
	pub fn with_clearing(
		x: u16,
		y: u16,
		bombs: u32,
		clearx: u16,
		cleary: u16,
	) -> Result<Self, NewBoardError> {
		let mut gb = Self::blank_board(x, y, bombs)?;

		if !((clearx < x) && (cleary < y)) {
			Err(NewBoardError::SizeConstraintOverflow)?;
		}

		gb.populate_without(clearx, cleary)?;

		Ok(gb)
	}

	/// generates a new board
	pub fn new(x: u16, y: u16, bombs: u32) -> Result<Self, NewBoardError> {
		let mut gb = Self::blank_board(x, y, bombs)?;

		gb.populate();

		Ok(gb)
	}

	/// creates a new board with the given bomb density on a scale of 0-1
	pub fn with_density(x: u16, y: u16, density: f64) -> Result<Self, NewBoardError> {
		if !(0. <= density && density <= 1.) {
			return Err(NewBoardError::BombOverflow);
		}

		let bombcount = (f64::from(widening_mul(x, y)) * density).round() as u32;

		Self::new(x, y, bombcount)
	}

	/// gets a specific tile on the board for public inspection
	pub fn get_board_tile(&self, x: u16, y: u16) -> Option<VisibleTile> {
		let (x, y) = widen_xy(x, y);

		let tile = self.board.get(y)?.get(x)?;
		Some(match tile.visible {
			Visibility::Visible => VisibleTile::Visible(tile.tile),
			Visibility::NotVisible => VisibleTile::NotVisible,
			Visibility::Flagged => VisibleTile::Flagged,
		})
	}

	/// makes all tiles visible, compromising a game boards state
	//#[cfg(debug_assertions)]
	pub unsafe fn make_visible(&mut self) {
		for x in self.board.iter_backing_mut() {
			x.visible = Visibility::Visible;
		}
	}

	//#[cfg(debug_assertions)]
	pub unsafe fn flag_bombs(&mut self) {
		for t in self.board.iter_backing_mut() {
			if let Tile::Bomb = t.tile {
				t.visible = Visibility::Flagged;
			}
		}
	}
	//#[cfg(debug_assertions)]
	pub unsafe fn cover_half(&mut self) {
		let mut rng = rand::thread_rng();

		for x in self.board.iter_backing_mut() {
			if rng.gen() && rng.gen() {
				x.visible = match x.visible {
					Visibility::Visible => Visibility::NotVisible,
					v @ _ => v,
				}
			}
		}
	}

	/// opens the 8 tiles around a tile
	pub fn open_around(&mut self, x: u16, y: u16) -> Result<Vec<(u16, u16)>, UnopenableError> {
		let openable = self.normalize_around_3x3(x, y);

		let mut opened = Vec::with_capacity(openable.len());

		let mut bombcnt = 0u32;

		for &(x, y) in openable.iter() {
			let tile = self.board[y][x];

			if tile.visible == Visibility::Flagged {
				bombcnt += 1;
			}
		}

		if bombcnt
			!= (self
				.tile_or_unopenable(x, y)?
				.tile
				.as_count()
				.ok_or(UnopenableError::BombHit)? as u32)
		{
			return Err(UnopenableError::FlagCountMismatch);
		}

		for &(x, y) in openable.iter() {
			let tile = self.board[y][x];

			match tile.visible {
				// ignore visible tiles
				Visibility::Visible => (),
				Visibility::NotVisible => {
					// if the notvisible tile we are trying to open is a bomb raise error
					assert_not_bomb(tile.tile)?;

					self.board[y][x].visible = Visibility::Visible;
					opened.push((x as u16, y as u16));
				}
				// don't attempt to open flagged tiles
				Visibility::Flagged => (),
			}
		}

		// open visible tiles to complete cycle
		opened.extend(self.open_visible());

		Ok(opened)
	}

	/// opens all visible tiles it sees, appends each coordinate to opened, and returns a final count of the amount of cells opened
	fn inner_open_visible(&mut self, opened: &mut Vec<(u16, u16)>) -> usize {
		let mut opened_count = 0usize;

		for y in 0..self.board.len() {
			for x in 0..self.board[y].len() {
				let tile = self.board[y][x];
				if tile.visible == Visibility::Visible && tile.tile == Tile::Zero {
					// runtime safety: all tiles around a tile are not bombs because the current tile is a Zero

					let mut open = vec![];

					for (x, y) in self.normalize_around_3x3(x as u16, y as u16) {
						let (x, y) = (x as u16, y as u16);
						match self.get(x, y).unwrap().visible {
							Visibility::NotVisible => {
								open.push((x, y));
								self.get_mut(x, y).unwrap().visible = Visibility::Visible;
							}
							_ => (),
						}
					}

					opened_count += open.len();
					opened.extend(open);
				}
			}
		}
		opened_count
	}

	/// opens all tiles that are naively open-able and returns the tiles that were opened
	pub fn open_visible(&mut self) -> Vec<(u16, u16)> {
		let mut out_arr = Vec::new();

		let mut per_iter = self.inner_open_visible(&mut out_arr);

		while per_iter != 0 {
			per_iter = self.inner_open_visible(&mut out_arr);
		}

		out_arr
	}

	/// opens the given tile
	pub fn open_tile(&mut self, x: u16, y: u16) -> Result<Vec<(u16, u16)>, UnopenableError> {
		let tile = self.tile_or_unopenable(x, y)?;
		let (x, y) = widen_xy(x, y);

		match tile.visible {
			Visibility::Visible => Err(UnopenableError::AlreadyOpen),
			Visibility::Flagged => Err(UnopenableError::FlaggedTile),
			Visibility::NotVisible => Ok(()),
		}?;

		if let Tile::Bomb = tile.tile {
			return Err(UnopenableError::BombHit);
		}

		// already confirmed bounds using get(y).get(x)
		self.board[y][x].visible = Visibility::Visible;

		let mut opened = self.open_visible();
		// include own tile
		opened.push((x as u16, y as u16));
		Ok(opened)
	}

	/// flags or unflags a tile depending on whether it is flagged already
	/// errors on an already open tile
	pub fn flag_tile(&mut self, x: u16, y: u16) -> Result<(), UnopenableError> {
		let tile = self.tile_or_unopenable(x, y)?;
		let (x, y) = widen_xy(x, y);

		match tile.visible {
			Visibility::Visible => Err(UnopenableError::AlreadyOpen),
			Visibility::Flagged => {
				self.board[y][x].visible = Visibility::NotVisible;
				Ok(())
			}
			Visibility::NotVisible => {
				self.board[y][x].visible = Visibility::Flagged;
				Ok(())
			}
		}
	}

	pub fn render(&self) -> FlatBoard<VisibleTile> {
		let (y, x) = self.board.dimensions();

		let mut board = FlatBoard::new(y, x, VisibleTile::NotVisible);

		let mut it = self.board.iter_backing();

		for j in board.iter_backing_mut() {
			let tile = it.next().expect("Sizes were not correctly constrained");

			*j = match tile.visible {
				Visibility::NotVisible => VisibleTile::NotVisible,
				Visibility::Visible => VisibleTile::Visible(tile.tile),
				Visibility::Flagged => VisibleTile::Flagged,
			};
		}

		board
	}

	/// undoes a move specified by a gameboard event
	pub fn undo_move(&mut self, event: &GameBoardEvent) -> Result<(), UndoError> {
		Ok(match event {
			GameBoardEvent::ToggleFlagCell(x, y) => self
				.get_mut(*x, *y)
				.ok_or(UndoError::OutOfBounds)?
				.swap_flag()
				.or(Err(UndoError::AlreadyOpen))?,
			GameBoardEvent::OpenCell(cells) => {
				for (x, y) in cells.iter().copied() {
					let tile = self.get_mut(x, y).ok_or(UndoError::OutOfBounds)?;

					if let Visibility::Visible = tile.visible {
						tile.visible = Visibility::NotVisible;
					} else {
						Err(UndoError::AlreadyClosed)?
					}
				}
			}
		})
	}
}

impl fmt::Display for GameBoard {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		const FLAG_CHAR: &str = "\u{2691}";

		for y in self.board.iter() {
			for x in y.iter() {
				use Visibility::*;
				match x.visible {
					Visible => write!(f, "\u{1b}[47m{}", x.tile)?,
					NotVisible => write!(f, "\u{1b}[30;107m  ")?,
					Flagged => write!(f, "\u{1b}[30;107m{FLAG_CHAR} ")?,
				}
			}
			write!(f, "\u{1b}[0m\n\u{1b}[107m")?;
		}

		write!(f, "\u{1b}[0m")
	}
}
