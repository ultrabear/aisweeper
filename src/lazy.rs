//! a lazy initialized game board
//!
//! This module exports the [LazyGameBoard], a wrapper around a [BaseGameBoard] that does not init the board until a move has been played.
//! This can be useful for wrapping [BaseGameBoard]'s that can only be created at the time of a move being made.

use super::gameboard::{
	BaseGameBoard, BaseGameBoard_do_event, FlatBoard, GameBoard, GameBoardEvent, KeyEvent,
	NewBoardError, UndoError, UnopenableError, VisibleTile,
};

pub struct LazyGameBoard<T: BaseGameBoard>(LazyGameBoardInner<T>);

// a lazy init logged game board that allows for init at any time and supports most methods for a game board
enum LazyGameBoardInner<T: BaseGameBoard> {
	Init(T),
	Uninit { x: u16, y: u16, bombs: u32 },
}

impl<T: BaseGameBoard> LazyGameBoard<T> {
	pub fn new_uninit(x: u16, y: u16, bombs: u32) -> Result<Self, NewBoardError> {
		GameBoard::validate_board(x, y, bombs, false, None)?;

		Ok(Self(Uninit { x, y, bombs }))
	}
}

// macro to either call a method on an init board or init a board with coordinates and then run the method
macro_rules! lazy_call {
	($se:ident, $fn_name:ident, $px:ident, $py:ident, $T:ty) => {
		match $se.0 {
			Init(ref mut board) => board.$fn_name($px, $py),
			Uninit { x, y, bombs } => {
				// hack to assert bounds before board creation crashes
				$se.get_board_tile($px, $py)
					.ok_or(UnopenableError::OutOfBounds)?;

				let mut b = <$T>::with_clearing(x, y, bombs, $px, $py).unwrap();

				let res = b.$fn_name($px, $py);

				$se.0 = Init(b);

				res
			}
		}
	};
}

use LazyGameBoardInner::{Init, Uninit};

impl<B: BaseGameBoard> BaseGameBoard for LazyGameBoard<B> {
	fn with_clearing(
		x: u16,
		y: u16,
		bombs: u32,
		clearx: u16,
		cleary: u16,
	) -> Result<Self, NewBoardError> {
		Ok(LazyGameBoard(Init(B::with_clearing(
			x, y, bombs, clearx, cleary,
		)?)))
	}

	fn flagged(&self) -> u32 {
		match self.0 {
			Init(ref board) => board.flagged(),
			Uninit { .. } => 0,
		}
	}

	fn opened(&self) -> u32 {
		match self.0 {
			Init(ref board) => board.opened(),
			Uninit { .. } => 0,
		}
	}

	fn open_around(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
		lazy_call!(self, open_around, x, y, B)
	}

	fn open_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
		lazy_call!(self, open_tile, x, y, B)
	}

	fn flag_tile(&mut self, x: u16, y: u16) -> Result<GameBoardEvent, UnopenableError> {
		lazy_call!(self, flag_tile, x, y, B)
	}

	fn lose_game(&mut self) {
		match self.0 {
			Init(ref mut board) => board.lose_game(),
			Uninit { .. } => {}
		}
	}

	fn win_game(&mut self) -> Result<(), u32> {
		match self.0 {
			Init(ref mut board) => board.win_game(),
			Uninit { .. } => Err(self.tiles_left()),
		}
	}

	fn undo_move(&mut self, ge: &GameBoardEvent) -> Result<(), UndoError> {
		match self.0 {
			Init(ref mut board) => board.undo_move(ge),
			Uninit { .. } => Err(UndoError::AlreadyClosed),
		}
	}

	fn dimensions(&self) -> (u16, u16) {
		match self.0 {
			Init(ref board) => board.dimensions(),
			Uninit { x, y, .. } => (x, y),
		}
	}

	fn bomb_count(&self) -> u32 {
		match self.0 {
			Init(ref board) => board.bomb_count(),
			Uninit { bombs, .. } => bombs,
		}
	}

	fn get_board_tile(&self, posx: u16, posy: u16) -> Option<VisibleTile> {
		match self.0 {
			Init(ref board) => board.get_board_tile(posx, posy),
			Uninit { x, y, .. } => {
				if (posx < x) && (posy < y) {
					Some(VisibleTile::NotVisible)
				} else {
					None
				}
			}
		}
	}

	fn render(&self) -> FlatBoard<VisibleTile> {
		match self.0 {
			Init(ref board) => board.render(),
			Uninit { x, y, .. } => FlatBoard::new((y).into(), (x).into(), VisibleTile::NotVisible),
		}
	}

	fn do_event(&mut self, ge: KeyEvent) -> Result<(), UnopenableError> {
		match self.0 {
			Init(ref mut board) => board.do_event(ge),
			Uninit { .. } => BaseGameBoard_do_event(self, ge),
		}
	}
}

impl<B: BaseGameBoard> LazyGameBoard<B> {
	fn init_with_mut(&mut self, clearx: u16, cleary: u16) -> &mut B {
		match self.0 {
			Init(ref mut board) => return board,
			Uninit { x, y, bombs } => {
				let board = B::with_clearing(x, y, bombs, clearx, cleary).unwrap();
				self.0 = Init(board);
			}
		}

		self.must_init_mut()
	}

	fn must_init_mut(&mut self) -> &mut B {
		match self.0 {
			Init(ref mut board) => board,
			Uninit { .. } => unreachable!(),
		}
	}

	fn must_init(&self) -> &B {
		match self.0 {
			Init(ref board) => board,
			Uninit { .. } => unreachable!(),
		}
	}
}
