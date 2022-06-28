#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
mod gameboard;
mod lazy;
mod logged;
mod ui;

use gameboard::GameBoard;
use lazy::LazyGameBoard;
use logged::LoggedGameBoard;
use ui::MineGameView;

use cursive::crossterm;

fn main() {
	let mut cursive = crossterm();

	let view: MineGameView<LazyGameBoard<LoggedGameBoard<GameBoard>>> =
		MineGameView::new_lazy(16, 16, 40).unwrap();

	cursive.add_layer(view);

	cursive.add_global_callback('q', |s| s.quit());

	cursive.set_fps(2);

	cursive.run();
}
