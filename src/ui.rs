use crate::types;
use types::{FlatBoard, GameBoard, NewBoardError, Tile, VisibleTile};

use cursive::{
	event,
	theme::{BaseColor, Color, ColorStyle, ColorType},
	view::View,
	Printer, XY,
};

use std::cell::Cell;

use super::{logged, logged::KeyEvent};

// a lazy init logged game board that allows for init at any time and supports most derived methods for a game board
enum LazyGameBoard {
	Init(logged::LoggedGameBoard),
	Uninit { x: u16, y: u16, bombs: u32 },
}

impl LazyGameBoard {
	fn init_with_mut(&mut self, clearx: u16, cleary: u16) -> &mut logged::LoggedGameBoard {
		use LazyGameBoard::*;

		match self {
			Init(board) => return board,
			Uninit { x, y, bombs } => {
				let board =
					logged::LoggedGameBoard::start_new(*x, *y, *bombs, clearx, cleary).unwrap();
				*self = Init(board);
			}
		}

		self.must_init_mut()
	}

	fn must_init_mut(&mut self) -> &mut logged::LoggedGameBoard {
		use LazyGameBoard::*;

		match self {
			Init(board) => board,
			Uninit { .. } => unreachable!(),
		}
	}

	fn must_init(&self, clearx: u16, cleary: u16) -> &logged::LoggedGameBoard {
		use LazyGameBoard::*;

		match self {
			Init(board) => board,
			Uninit { .. } => unreachable!(),
		}
	}

	fn area(&self) -> u32 {
		use LazyGameBoard::*;
		match self {
			Init(board) => board.area(),
			Uninit { x, y, .. } => u32::from(*x) * u32::from(*y),
		}
	}

	fn dimensions(&self) -> (u16, u16) {
		use LazyGameBoard::*;
		match self {
			Init(board) => board.dimensions(),
			Uninit { x, y, .. } => (*x, *y),
		}
	}

	fn bomb_count(&self) -> u32 {
		use LazyGameBoard::*;
		match self {
			Init(board) => board.bomb_count(),
			Uninit { bombs, .. } => *bombs,
		}
	}

	fn render(&self) -> FlatBoard<VisibleTile> {
		use LazyGameBoard::*;

		match self {
			Init(board) => board.render(),
			Uninit { x, y, .. } => {
				FlatBoard::new((*y).into(), (*x).into(), VisibleTile::NotVisible)
			}
		}
	}
}

pub struct MineGameView {
	board: LazyGameBoard,
}

impl MineGameView {
	pub fn new(x: u16, y: u16, bombs: u32) -> Result<Self, NewBoardError> {
		GameBoard::validate_board(x, y, bombs, false, None)?;

		Ok(Self {
			board: LazyGameBoard::Uninit { x, y, bombs },
		})
	}
}

fn visible_tile_to_cursive(v: VisibleTile) -> (ColorStyle, String) {
	macro_rules! tty_color {
		($lightness:ident::$color:ident) => {
			ColorType::Color(Color::$lightness(BaseColor::$color))
		};
	}

	fn colorof(tile: Tile) -> ColorType {
		match tile {
			Tile::Zero => tty_color!(Dark::Black),
			Tile::One => tty_color!(Light::Blue),
			Tile::Two => tty_color!(Light::Green),
			Tile::Three => tty_color!(Light::Red),
			Tile::Four => tty_color!(Dark::Blue),
			Tile::Five => tty_color!(Dark::Red),
			Tile::Six => tty_color!(Dark::Cyan),
			Tile::Seven => tty_color!(Dark::Black),
			Tile::Eight => tty_color!(Light::Black),
			Tile::Bomb => tty_color!(Dark::Red),
		}
	}

	let covered_tile_backing = tty_color!(Light::White);
	let uncovered_tile_backing = tty_color!(Dark::White);
	let black = tty_color!(Dark::Black);

	let basic_color = ColorStyle {
		front: black,
		back: covered_tile_backing,
	};

	const FLAG_CHAR: &str = "\u{2691} ";

	match v {
		VisibleTile::Visible(tile) => (
			ColorStyle {
				front: colorof(tile),
				back: uncovered_tile_backing,
			},
			format!("{} ", tile.as_str_count()),
		),
		VisibleTile::NotVisible => (basic_color, String::from("  ")),
		VisibleTile::Flagged => (basic_color, String::from(FLAG_CHAR)),
	}
}

impl View for MineGameView {
	fn draw(&self, p: &Printer<'_, '_>) {
		p.print((0usize, 0), format!("{}", self.board.bomb_count()).as_str());

		let base_render = self.board.render();

		for (y_idx, y) in base_render.iter().enumerate() {
			for (x_idx, x) in y.into_iter().enumerate() {
				let (style, string) = visible_tile_to_cursive(*x);

				p.with_color(style, |colored_print| {
					colored_print.print((x_idx * 2, y_idx + 1), string.as_str())
				});
			}
		}
	}

	fn required_size(&mut self, _: XY<usize>) -> XY<usize> {
		let (x, y) = self.board.dimensions();

		XY {
			x: usize::from(x) * 2,
			y: usize::from(y) + 1usize,
		}
	}

	fn on_event(&mut self, e: event::Event) -> event::EventResult {
		use std::io::prelude::*;
		let mut elog = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.append(true)
			.open("elog.txt")
			.unwrap();

		let log_err = |e| elog.write(format!("{:?} {}\n", e, e).as_bytes()).unwrap();

		use event::{Event, EventResult, MouseButton, MouseEvent};

		match e {
			Event::Mouse {
				position,
				event,
				offset,
			} => {
				if ((position.y as isize) - (offset.y as isize)) <= 0 {
					return EventResult::Ignored;
				} else if ((position.x as isize) - (offset.x as isize)) < 0 {
					return EventResult::Ignored;
				}

				let board_p = (
					((position.x - offset.x) / 2) as u16,
					((position.y - offset.y) - 1) as u16,
				);

				match event {
					MouseEvent::Press(b) => match b {
						MouseButton::Left => {
							let _ = self
								.board
								.init_with_mut(board_p.0, board_p.1)
								.do_event(KeyEvent::Mouse1(board_p.0, board_p.1))
								.map_err(log_err);
							EventResult::Consumed(None)
						}
						MouseButton::Right => match &mut self.board {
							LazyGameBoard::Init(board) => {
								let _ = board
									.do_event(KeyEvent::Mouse2(board_p.0, board_p.1))
									.map_err(log_err);
								EventResult::Consumed(None)
							}
							LazyGameBoard::Uninit { .. } => EventResult::Ignored,
						},
						_ => EventResult::Ignored,
					},
					_ => EventResult::Ignored,
				}
			}
			_ => EventResult::Ignored,
		}
	}
}
