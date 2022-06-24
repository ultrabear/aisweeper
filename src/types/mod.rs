//types repo

mod tiles;
pub use tiles::{Tile, VisibleTile};

mod errors;
pub use errors::{NewBoardError, UnopenableError};

mod gameboard;
pub use gameboard::{FlatBoard, GameBoard, GameBoardEvent};
