use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Tile {
	Zero = 0,
	One,
	Two,
	Three,
	Four,
	Five,
	Six,
	Seven,
	Eight,
	Bomb,
}

impl Tile {
	/// returns true if tile is of variant Bomb
	pub const fn is_bomb(self) -> bool {
		matches!(self, Tile::Bomb)
	}

	/// Returns self as count of bombs in 8 surrounding squares, or None if is a bomb
	pub const fn as_count(self) -> Option<u8> {
		match self {
			Tile::Bomb => None,
			v => Some(v as u8),
		}
	}

	/// returns count as a single width string, where a bomb is represented by a B
	/// used as a pre end user stage for naked processing
	pub const fn as_str_count(self) -> &'static str {
		match self {
			Tile::Zero => " ",
			Tile::One => "1",
			Tile::Two => "2",
			Tile::Three => "3",
			Tile::Four => "4",
			Tile::Five => "5",
			Tile::Six => "6",
			Tile::Seven => "7",
			Tile::Eight => "8",
			Tile::Bomb => "B",
		}
	}
}

impl fmt::Display for Tile {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use Tile::*;

		const C: &str = "\u{1b}[";

		match self {
			Zero => write!(f, "  "),
			One => write!(f, "{C}94m1 "),
			Two => write!(f, "{C}92m2 "),
			Three => write!(f, "{C}91m3 "),
			Four => write!(f, "{C}34m4 "),
			Five => write!(f, "{C}31m5 "),
			Six => write!(f, "{C}36m6 "),
			Seven => write!(f, "{C}30m7 "),
			Eight => write!(f, "{C}90m8 "),
			Bomb => write!(f, "{C}31mB "),
		}
	}
}

impl TryFrom<u8> for Tile {
	type Error = ();

	fn try_from(v: u8) -> Result<Self, Self::Error> {
		use Tile::*;

		Ok(match v {
			0 => Zero,
			1 => One,
			2 => Two,
			3 => Three,
			4 => Four,
			5 => Five,
			6 => Six,
			7 => Seven,
			8 => Eight,
			_ => return Err(()),
		})
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum Visibility {
	Visible,
	NotVisible,
	Flagged,
}

#[derive(Copy, Clone, Debug)]
pub(super) struct BoardTile {
	pub(super) tile: Tile,
	pub(super) visible: Visibility,
}

pub(super) struct AlreadyOpen;

impl BoardTile {
	pub(super) fn swap_flag(&mut self) -> Result<(), AlreadyOpen> {
		match self.visible {
			Visibility::Visible => Err(AlreadyOpen)?,
			Visibility::NotVisible => self.visible = Visibility::Flagged,
			Visibility::Flagged => self.visible = Visibility::NotVisible,
		}
		Ok(())
	}
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum VisibleTile {
	NotVisible,
	Visible(Tile),
	Flagged,
}
