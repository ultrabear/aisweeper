use super::errors::{NewBoardError, UndoError, UnopenableError};
use super::flatboard::{FlatBoard, IterBackingMut};
use super::tiles::VisibleTile;

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
	pub const fn flag_tile(x: u16, y: u16) -> Self {
		Self::ToggleFlagCell(x, y)
	}
}

#[inline]
fn widening_mul(a: u16, b: u16) -> u32 {
	u32::from(a) * u32::from(b)
}

#[derive(Copy, Clone, Debug)]
pub enum KeyEvent {
	Mouse1(u16, u16),
	Mouse2(u16, u16),
	Pause,
	UnPause,
	Idle,
}

#[allow(non_snake_case)]
pub fn BaseGameBoard_do_event<T: BaseGameBoard>(
	t: &mut T,
	k: KeyEvent,
) -> Result<(), UnopenableError> {
	use KeyEvent::*;

	match k {
		Mouse1(x, y) => {
			let tile = t.get_board_tile(x, y).ok_or(UnopenableError::OutOfBounds)?;

			match tile {
				VisibleTile::NotVisible => {
					t.open_tile(x, y)?;
				}
				VisibleTile::Visible(_) => {
					t.open_around(x, y)?;
				}
				VisibleTile::Flagged => {}
			}
		}
		Mouse2(x, y) => {
			let tile = t.get_board_tile(x, y).ok_or(UnopenableError::OutOfBounds)?;

			match tile {
				VisibleTile::NotVisible => {
					t.flag_tile(x, y)?;
				}
				VisibleTile::Flagged => {
					t.flag_tile(x, y)?;
				}
				VisibleTile::Visible(_) => {}
			}
		}
		_ => {}
	}

	Ok(())
}

pub trait BaseGameBoard: Sized {
	/// returns the dimensions of the board in x/y form
	fn dimensions(&self) -> (u16, u16);
	/// returns the count of bombs in this board
	fn bomb_count(&self) -> u32;

	/// generates a new board with a given 3x3 clear zone
	fn with_clearing(
		x: u16,
		y: u16,
		bombs: u32,
		clear_x: u16,
		clear_y: u16,
	) -> Result<Self, NewBoardError>;

	/// opens a tile
	fn open_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError>;
	/// opens the 8 tiles surrounding a tile
	fn open_around(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError>;
	/// flags or unflags a given tile
	fn flag_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError>;

	/// undoes a move in the board state specified by a GameBoardEvent
	fn undo_move(&mut self, event: &GameBoardEvent) -> Result<(), UndoError>;

	/// gets a tile on the board
	fn get_board_tile(&self, x: u16, y: u16) -> Option<VisibleTile>;

	/// processes a KeyEvent using mouse 1/2, default impl ignores other events
	fn do_event(&mut self, k: KeyEvent) -> Result<(), UnopenableError> {
		BaseGameBoard_do_event(self, k)
	}

	/// returns a FlatBoard of the board rendered as y/x in terms of VisibleTile's
	fn render(&self) -> FlatBoard<VisibleTile> {
		let mut board = FlatBoard::new(
			self.get_y().into(),
			self.get_x().into(),
			VisibleTile::NotVisible,
		);

		let mut it = board.iter_backing_mut();

		for y in 0..self.get_y() {
			for x in 0..self.get_x() {
				let j = it.next().unwrap();

				*j = self.get_board_tile(x, y).unwrap();
			}
		}

		board
	}

	/// returns the computed x*y area of a game board with no possibility of overflow
	#[inline]
	fn area(&self) -> u32 {
		widening_mul(self.get_x(), self.get_y())
	}

	/// returns the x dimension of a game board
	#[inline]
	fn get_x(&self) -> u16 {
		self.dimensions().0
	}

	/// returns the y dimension of a game board
	#[inline]
	fn get_y(&self) -> u16 {
		self.dimensions().1
	}

	/// returns the bomb density as a float in the range [0,1]
	#[inline]
	fn bomb_density(&self) -> f64 {
		f64::from(self.bomb_count()) / f64::from(self.area())
	}
}
