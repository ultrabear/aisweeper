mod logged;
mod types;
mod ui;

use ui::MineGameView;

use logged::LoggedGameBoard;

use cursive::crossterm;

fn main() {
	let mut cursive = crossterm();

	cursive.add_layer(MineGameView::new(16, 16, 40).unwrap());

	cursive.add_global_callback('q', |s| s.quit());

	cursive.set_fps(2);

	cursive.run();
}
