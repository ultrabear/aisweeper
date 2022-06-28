use std::fmt;
use std::iter::repeat;

use rand::prelude::*;

mod tiles;
use tiles::{BoardTile, Visibility};

pub use tiles::{Tile, VisibleTile};

mod errors;
use errors::assert_not_bomb;
pub use errors::{NewBoardError, UndoError, UnopenableError};

mod flatboard;
pub use flatboard::{FlatBoard, IterBacking, IterBackingMut};

mod interface;
pub use interface::{BaseGameBoard, BaseGameBoard_do_event, GameBoardEvent, KeyEvent};

#[derive(Debug)]
pub struct GameBoard {
	pub(self) bombs: u32,
	// board is indexed as y/x but the api uses x/y
	pub(self) board: FlatBoard<BoardTile>,
}

#[inline]
fn widening_mul(a: u16, b: u16) -> u32 {
	u32::from(a) * u32::from(b)
}

fn widen_xy<T: From<S>, S>(x: S, y: S) -> (T, T) {
	(x.into(), y.into())
}

/// basic utils
impl GameBoard {
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

		if (self.area() - self.bombs) < 9 {
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
	/// assumes precondition of a valid board config
	fn blank_board(x: u16, y: u16, bombs: u32) -> Self {
		Self {
			bombs,
			board: FlatBoard::new(
				y.into(),
				x.into(),
				BoardTile {
					tile: Tile::Zero,
					visible: Visibility::NotVisible,
				},
			),
		}
	}

	/// validation method for a new board, able to run publically for validating board configs before more costly generation
	/// it is in theory safe to call unwrap_unchecked on associated new methods if this function does not return Err with the same configuration, however this is not recommended
	pub fn validate_board(
		x: u16,
		y: u16,
		bombs: u32,
		has_clearing: bool,
		clearing_coordinates: impl Into<Option<(u16, u16)>>,
	) -> Result<(), NewBoardError> {
		Self::validate_size_constraints(x, y, bombs)?;

		let area: u32 = widening_mul(x, y);
		if area < bombs {
			return Err(NewBoardError::BombOverflow);
		}

		if x == 0 || y == 0 {
			return Err(NewBoardError::ZeroDimension);
		}

		if let Some((clearx, cleary)) = clearing_coordinates.into() {
			if !((clearx < x) && (cleary < y)) {
				return Err(NewBoardError::SizeConstraintOverflow);
			}
		}

		if has_clearing {
			if (area - bombs) < 9 {
				return Err(NewBoardError::BombOverflow);
			}
		}

		Ok(())
	}

	/// generates a new board
	pub fn new(x: u16, y: u16, bombs: u32) -> Result<Self, NewBoardError> {
		Self::validate_board(x, y, bombs, false, None)?;
		let mut gb = Self::blank_board(x, y, bombs);

		gb.populate();

		Ok(gb)
	}

	/// opens all visible tiles it sees, appends each coordinate to opened, and returns a final count of the amount of cells opened
	fn inner_open_visible(&mut self, opened: &mut Vec<(u16, u16)>) -> usize {
		let mut opened_count = 0usize;

		for y in 0..self.board.len() {
			for x in 0..self.board[y].len() {
				let tile = self.board[y][x];
				if tile.visible == Visibility::Visible && tile.tile == Tile::Zero {
					for (x, y) in self.normalize_around_3x3(x as u16, y as u16) {
						let (x, y) = (x as u16, y as u16);
						match self.get(x, y).unwrap().visible {
							Visibility::NotVisible => {
								opened.push((x, y));
								opened_count += 1;
								// SAFETY: all tiles around a tile are not bombs because the current tile is a Zero, so overwrite with a Visible
								self.get_mut(x, y).unwrap().visible = Visibility::Visible;
							}
							_ => (),
						}
					}
				}
			}
		}
		opened_count
	}

	/// opens all tiles that are naively open-able and stores in out_arr
	fn open_visible(&mut self, out_arr: &mut Vec<(u16, u16)>) {
		let mut per_iter = self.inner_open_visible(out_arr);

		while per_iter != 0 {
			per_iter = self.inner_open_visible(out_arr);
		}
	}
}

impl BaseGameBoard for GameBoard {
	fn bomb_count(&self) -> u32 {
		self.bombs
	}

	fn dimensions(&self) -> (u16, u16) {
		(
			self.board.dimensions().1.try_into().unwrap(),
			self.board.len().try_into().unwrap(),
		)
	}

	/// generates a new board with a given clear zone where no bombs will be guaranteed
	fn with_clearing(
		x: u16,
		y: u16,
		bombs: u32,
		clearx: u16,
		cleary: u16,
	) -> Result<Self, NewBoardError> {
		Self::validate_board(x, y, bombs, true, (clearx, cleary))?;

		let mut gb = Self::blank_board(x, y, bombs);

		gb.populate_without(clearx, cleary)?;

		Ok(gb)
	}

	/// opens the 8 tiles around a tile
	fn open_around(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
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
		self.open_visible(&mut opened);

		Ok(opened.into())
	}

	/// opens the given tile
	fn open_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
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

		let mut opened = Vec::new();
		self.open_visible(&mut opened);
		// include own tile
		opened.push((x as u16, y as u16));

		Ok(opened.into())
	}

	/// flags or unflags a tile depending on whether it is flagged already
	/// errors on an already open tile
	fn flag_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
		let tile = self.tile_or_unopenable(x, y)?;
		let (bx, by) = widen_xy(x, y);

		match tile.visible {
			Visibility::Visible => Err(UnopenableError::AlreadyOpen),
			Visibility::Flagged => {
				self.board[by][bx].visible = Visibility::NotVisible;
				Ok(GameBoardEvent::flag_tile(x, y))
			}
			Visibility::NotVisible => {
				self.board[by][bx].visible = Visibility::Flagged;
				Ok(GameBoardEvent::flag_tile(x, y))
			}
		}
	}

	/// gets a specific tile on the board for public inspection
	fn get_board_tile(&self, x: u16, y: u16) -> Option<VisibleTile> {
		let (x, y) = widen_xy(x, y);

		let tile = self.board.get(y)?.get(x)?;
		Some(match tile.visible {
			Visibility::Visible => VisibleTile::Visible(tile.tile),
			Visibility::NotVisible => VisibleTile::NotVisible,
			Visibility::Flagged => VisibleTile::Flagged,
		})
	}

	/// undoes a move specified by a gameboard event
	fn undo_move(&mut self, event: &GameBoardEvent) -> Result<(), UndoError> {
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

	fn render(&self) -> FlatBoard<VisibleTile> {
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
}
