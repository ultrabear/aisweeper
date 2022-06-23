mod types;
use types::GameBoard;

mod ui;
use ui::MineGameView;

mod logged;
use logged::LoggedGameBoard;

use cursive::crossterm;

mod input;

fn test_a_board() {
	let mut gb = GameBoard::with_clearing(30, 17, 99, 5, 5)
		.map_err(|e| {
			eprintln!("{}", e);
			e
		})
		.unwrap();

	unsafe {
		gb.flag_bombs();
		gb.cover_half();
	}
	gb.open_tile(5, 5);

	println!("{gb}");

	println!("density: {:.1}%", gb.bomb_density() * 100f64);
}

macro_rules! timeit {
	($callable:ident) => {{
		use std::time::Instant;

		let tstart = Instant::now();

		$callable();

		println!("Took: {:.2}ms", tstart.elapsed().as_micros() as f64 / 1000.);
	}};
}

fn main() {
	let mut cursive = crossterm();

	let mut gb = LoggedGameBoard::start_new(30, 16, 99, 5, 5).unwrap();

	cursive.add_layer(MineGameView::new(gb));

	cursive.add_global_callback('q', |s| s.quit());

  cursive.set_fps(3);

	cursive.run();
}
