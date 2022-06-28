use super::tiles::Tile;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NewBoardError {
	#[error("the count of bombs was greater than the size of the board, or was negative")]
	BombOverflow,
	#[error("one or more passed dimensions was zero")]
	ZeroDimension,
	#[error("exceeded one or more dimensional limits (10k max x/y, 100m max bombs), or clearing zone was out of bounds")]
	SizeConstraintOverflow,
}

#[derive(Error, Debug)]
pub enum UnopenableError {
	#[error("a bomb was under this tile")]
	BombHit,
	#[error("this tile is already open")]
	AlreadyOpen,
	#[error("this tile is flagged")]
	FlaggedTile,
	#[error("this tile is out of bounds")]
	OutOfBounds,
	#[error("flag count does not match count of tile")]
	FlagCountMismatch,
}

#[derive(Error, Debug)]
pub enum UndoError {
	#[error("a tile is out of bounds")]
	OutOfBounds,
	#[error("this tile is already closed, cannot unopen")]
	AlreadyClosed,
	#[error("this tile is open, cannot toggle flag")]
	AlreadyOpen,
}

pub const fn assert_not_bomb(t: Tile) -> Result<(), UnopenableError> {
	match t {
		Tile::Bomb => Err(UnopenableError::BombHit),
		_ => Ok(()),
	}
}
