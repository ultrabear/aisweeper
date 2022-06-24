mod logged;
mod types;
mod ui;

use ui::MineGameView;

use logged::LoggedGameBoard;

use cursive::crossterm;

fn main() {
	let mut cursive = crossterm();

	let gb = LoggedGameBoard::start_new(30, 16, 99, 5, 5).unwrap();

	cursive.add_layer(MineGameView::new(gb));

	cursive.add_global_callback('q', |s| s.quit());

	cursive.set_fps(3);

	cursive.run();
}
