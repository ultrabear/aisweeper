/// main ui interactions, houses rendering for game view and integrations with cursive
use crate::gameboard;
use gameboard::{BaseGameBoard, GameBoard, KeyEvent, NewBoardError, Tile, VisibleTile};

use cursive::{
	event,
	theme::{BaseColor, Color, ColorStyle, ColorType},
	view::View,
	Printer, XY,
};

use crate::lazy::LazyGameBoard;

pub struct MineGameView<T: BaseGameBoard> {
	board: T,
}

impl<T: BaseGameBoard> MineGameView<LazyGameBoard<T>> {
	pub fn new_lazy(x: u16, y: u16, bombs: u32) -> Result<Self, NewBoardError> {
		GameBoard::validate_board(x, y, bombs, false, None)?;

		Ok(Self {
			board: LazyGameBoard::new_uninit(x, y, bombs).unwrap(),
		})
	}
}

fn visible_tile_to_cursive(v: VisibleTile) -> (ColorStyle, String) {
	macro_rules! tty_color {
		($lightness:ident::$color:ident) => {
			ColorType::Color(Color::$lightness(BaseColor::$color))
		};
	}

	const fn colorof(tile: Tile) -> ColorType {
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

impl<B: BaseGameBoard + 'static> View for MineGameView<B> {
	fn draw(&self, p: &Printer<'_, '_>) {
		p.print((0usize, 0), format!("{}", self.board.bomb_count()).as_str());

		let base_render = self.board.render();

		for (y_idx, y) in base_render.iter().enumerate() {
			for (x_idx, x) in y.iter().enumerate() {
				let (style, string) = visible_tile_to_cursive(*x);

				p.with_color(style, |colored_print| {
					colored_print.print((x_idx * 2, y_idx + 1), string.as_str());
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
				if ((position.y as isize) - (offset.y as isize)) <= 0
					|| ((position.x as isize) - (offset.x as isize)) < 0
				{
					return EventResult::Ignored;
				}

				let board_p: (u16, u16) = (|| {
					Some((
						((position.x - offset.x) / 2).try_into().ok()?,
						((position.y - offset.y) - 1).try_into().ok()?,
					))
				})()
				.unwrap_or((u16::MAX, u16::MAX));

				match event {
					MouseEvent::Press(b) => match b {
						MouseButton::Left => {
							let _ = self
								.board
								.do_event(KeyEvent::Mouse1(board_p.0, board_p.1))
								.map_err(log_err);
							EventResult::Consumed(None)
						}
						MouseButton::Right => {
							let _ = self
								.board
								.do_event(KeyEvent::Mouse2(board_p.0, board_p.1))
								.map_err(log_err);
							EventResult::Consumed(None)
						}
						_ => EventResult::Ignored,
					},
					_ => EventResult::Ignored,
				}
			}
			_ => EventResult::Ignored,
		}
	}
}
