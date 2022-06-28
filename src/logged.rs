//! a logged game board implementation that stores every move
//!
//! defines the [`KeyEvent`](enum.KeyEvent.html) and [`LoggedGameBoard`](struct.LoggedGameBoard.html), structures for logging and storing keystrokes and the logging implementation itself
//! most methods are re exported from [`GameBoard`](../types/gameboard/struct.GameBoard.html), with move success details being rerouted to the logging system

use super::gameboard;

use gameboard::{
	BaseGameBoard, FlatBoard, GameBoardEvent, KeyEvent, NewBoardError, UndoError, UnopenableError,
	VisibleTile,
};

use time;

/// internally stored keyevent that also stores any effect it had on the gameboard
enum KeyEventEffect {
	Mouse1(u16, u16, GameBoardEvent),
	Mouse2(u16, u16, GameBoardEvent),
	Pause,
	UnPause,
	Idle,
}

#[derive(Debug)]
struct RequiresGameBoardEvent;

impl TryFrom<KeyEvent> for KeyEventEffect {
	type Error = RequiresGameBoardEvent;

	fn try_from(k: KeyEvent) -> Result<Self, Self::Error> {
		match k {
			KeyEvent::Mouse1(_, _) => Err(RequiresGameBoardEvent),
			KeyEvent::Mouse2(_, _) => Err(RequiresGameBoardEvent),
			KeyEvent::Pause => Ok(KeyEventEffect::Pause),
			KeyEvent::UnPause => Ok(KeyEventEffect::UnPause),
			KeyEvent::Idle => Ok(KeyEventEffect::Idle),
		}
	}
}

struct LogFrame {
	time_offset_micros: u64,
	trace: KeyEventEffect,
}

pub struct LoggedGameBoard<GB: BaseGameBoard> {
	start_time: time::OffsetDateTime,
	start_mono: time::Instant,

	board: GB,

	events: Vec<LogFrame>,
}

impl<T: BaseGameBoard> LoggedGameBoard<T> {
	pub fn start_new(
		x: u16,
		y: u16,
		bombs: u32,
		opening_x: u16,
		opening_y: u16,
	) -> Result<Self, NewBoardError> {
		let mut board = Self {
			start_time: time::OffsetDateTime::now_utc(),
			start_mono: time::Instant::now(),
			board: T::with_clearing(x, y, bombs, opening_x, opening_y)?,
			events: vec![],
		};

		board.events.push(LogFrame {
			// SAFETY: GameBoard::with_clearing guarantees that clearx and cleary are an empty square, so no bomb is possible
			// bounds are checked via with_clearing validating bounds on clearx and cleary
			// cell is not opened/flagged because we just created a new board
			trace: KeyEventEffect::Mouse1(
				x,
				y,
				board.board.open_tile(opening_x, opening_y).unwrap().into(),
			),
			time_offset_micros: board.current_micros_offset(),
		});

		Ok(board)
	}

	fn current_micros_offset(&self) -> u64 {
		self.start_mono.elapsed().whole_microseconds().try_into().expect("Game timer exceeded 64 bit limit of microseconds (exceeding 200_000 years since game start)")
	}
}

impl<T: BaseGameBoard> BaseGameBoard for LoggedGameBoard<T> {
	fn with_clearing(
		x: u16,
		y: u16,
		bombs: u32,
		clearx: u16,
		cleary: u16,
	) -> Result<Self, NewBoardError> {
		Self::start_new(x, y, bombs, clearx, cleary)
	}

	fn bomb_count(&self) -> u32 {
		self.board.bomb_count()
	}
	fn dimensions(&self) -> (u16, u16) {
		self.board.dimensions()
	}

	fn get_board_tile(&self, x: u16, y: u16) -> Option<VisibleTile> {
		self.board.get_board_tile(x, y)
	}

	fn open_around(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
		self.board.open_around(x, y)
	}

	fn open_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
		self.board.open_tile(x, y)
	}

	fn flag_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
		self.board.flag_tile(x, y)
	}

	fn render(&self) -> FlatBoard<VisibleTile> {
		self.board.render()
	}

	fn undo_move(&mut self, f: &GameBoardEvent) -> Result<(), UndoError> {
		self.board.undo_move(f)
	}

	fn do_event(&mut self, k: KeyEvent) -> Result<(), UnopenableError> {
		{
			use KeyEvent::*;
			match k {
				Mouse1(x, y) => {
					let tile = self
						.get_board_tile(x, y)
						.ok_or(UnopenableError::OutOfBounds)?;
					let event: GameBoardEvent = match tile {
						VisibleTile::NotVisible => self.board.open_tile(x, y)?.into(),
						VisibleTile::Visible(_) => self.board.open_around(x, y)?.into(),
						VisibleTile::Flagged => {
							return Err(UnopenableError::FlaggedTile);
						}
					};

					self.events.push(LogFrame {
						trace: KeyEventEffect::Mouse1(x, y, event),
						time_offset_micros: self.current_micros_offset(),
					});
				}
				Mouse2(x, y) => {
					let tile = self
						.get_board_tile(x, y)
						.ok_or(UnopenableError::OutOfBounds)?;
					match tile {
						VisibleTile::NotVisible => self.board.flag_tile(x, y)?,
						VisibleTile::Flagged => self.board.flag_tile(x, y)?,
						VisibleTile::Visible(_) => {
							return Err(UnopenableError::AlreadyOpen);
						}
					};

					self.events.push(LogFrame {
						trace: KeyEventEffect::Mouse2(x, y, GameBoardEvent::flag_tile(x, y)),
						time_offset_micros: self.current_micros_offset(),
					});
				}
				v @ _ => self.events.push(LogFrame {
					trace: v.try_into().expect(
						"Impossible invariant (KeyEvent Mouse1 and Mouse2 already handled)",
					),
					time_offset_micros: self.current_micros_offset(),
				}),
			};
		}

		Ok(())
	}
}
