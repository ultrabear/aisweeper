//! Base errors that a [BaseGameBoard][super::BaseGameBoard] can return

use super::tiles::Tile;
use thiserror::Error;

/// an error returned when creation of a new board fails
#[derive(Error, Debug)]
pub enum NewBoardError {
	#[error("the count of bombs was greater than the size of the board, or was negative")]
	BombOverflow,
	#[error("one or more passed dimensions was zero")]
	ZeroDimension,
	#[error("exceeded one or more dimensional limits (10k max x/y, 100m max bombs), or clearing zone was out of bounds")]
	SizeConstraintOverflow,
}

/// an error returned when during normal play an exception is reached, which may or may not be a game over state
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
	#[error("game has already ended")]
	GameOver,
}

/// an error returned when the [BaseGameBoard][super::BaseGameBoard] failed to undo a move
#[derive(Error, Debug)]
pub enum UndoError {
	#[error("a tile is out of bounds")]
	OutOfBounds,
	#[error("this tile is already closed, cannot unopen")]
	AlreadyClosed,
	#[error("this tile is open, cannot toggle flag")]
	AlreadyOpen,
}

/// returns a [UnopenableError::BombHit] if the tile is a bomb
pub const fn assert_not_bomb(t: Tile) -> Result<(), UnopenableError> {
	match t.is_bomb() {
		true => Err(UnopenableError::BombHit),
		false => Ok(()),
	}
}
