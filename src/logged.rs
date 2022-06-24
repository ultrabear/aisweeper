use super::types;

use types::{FlatBoard, GameBoard, GameBoardEvent, NewBoardError, UnopenableError, VisibleTile};

use time;

#[derive(Copy, Clone, Debug)]
pub enum KeyEvent {
	Mouse1(u16, u16),
	Mouse2(u16, u16),
	Pause,
	UnPause,
	Idle,
}

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

pub struct LoggedGameBoard {
	start_time: time::OffsetDateTime,
	start_mono: time::Instant,

	board: GameBoard,

	events: Vec<LogFrame>,
}

impl LoggedGameBoard {
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
			board: GameBoard::with_clearing(x, y, bombs, opening_x, opening_y)?,
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

impl LoggedGameBoard {
	pub fn bomb_count(&self) -> u32 {
		self.board.bomb_count()
	}
	pub fn bomb_density(&self) -> f64 {
		self.board.bomb_density()
	}

	pub fn dimensions(&self) -> (u16, u16) {
		self.board.dimensions()
	}

	pub fn area(&self) -> u32 {
		self.board.area()
	}

	pub fn get_board_tile(&self, x: u16, y: u16) -> Option<VisibleTile> {
		self.board.get_board_tile(x, y)
	}

	pub fn open_around(&mut self, x: u16, y: u16) -> Result<(), UnopenableError> {
		let res = self.board.open_around(x, y)?;

		self.events.push(LogFrame {
			trace: KeyEventEffect::Mouse1(x, y, res.into()),
			time_offset_micros: self.current_micros_offset(),
		});

		Ok(())
	}

	pub fn open_tile(&mut self, x: u16, y: u16) -> Result<(), UnopenableError> {
		let res = self.board.open_tile(x, y)?;

		self.events.push(LogFrame {
			trace: KeyEventEffect::Mouse1(x, y, res.into()),
			time_offset_micros: self.current_micros_offset(),
		});

		Ok(())
	}
	pub fn flag_tile(&mut self, x: u16, y: u16) -> Result<(), UnopenableError> {
		self.board.flag_tile(x, y)?;

		self.events.push(LogFrame {
			trace: KeyEventEffect::Mouse2(x, y, GameBoardEvent::flag_cell(x, y)),
			time_offset_micros: self.current_micros_offset(),
		});

		Ok(())
	}

	pub fn do_event(&mut self, k: KeyEvent) -> Result<(), UnopenableError> {
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
						trace: KeyEventEffect::Mouse2(x, y, GameBoardEvent::flag_cell(x, y)),
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

	pub fn render(&self) -> FlatBoard<VisibleTile> {
		self.board.render()
	}
}
