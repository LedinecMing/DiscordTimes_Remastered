pub struct Pos(pub usize);
impl Pos {
	pub fn from_xy(board: &impl Board, xy: (usize, usize)) -> Self {
		let size = board.get_size();
		Self( xy.0 + xy.1 * size.0 )
	}
	pub fn into_xy(&self, board: &impl Board) -> (usize, usize) {
		let size = board.get_size();
		(self.0 % size.0, self.0 / size.0)
	}
}
pub struct Move(pub Pos, pub Pos);

pub enum Side {
	First,
	Second
}

pub trait Board {
	fn make_move(&mut self, mov: Move);
	fn get_possible_moves(&self) -> Vec<Move>;
	fn calculate_winner(&self) -> (Option<Side>, f64);
	fn get_size(&self) -> (usize, usize);
}

mod tests {
	use super::*;
	#[derive(Clone)]
	struct Bitboard(pub u16);
	#[derive(Clone)]
	struct Pieces(pub Bitboard, pub Bitboard);
	#[derive(Clone)]
	struct MovementGame {
		pub board: Pieces
	}
	impl MovementGame {
		fn new(board: Pieces) -> Self {
			Self { board }
		}
	}
	impl Board for MovementGame {
		fn make_move(&mut self, mov: Move) {
			todo!()
		}
		fn calculate_winner(&self) -> (Option<Side>, f64) {
			let a = self.board.0.0 & 0b0000000000001111;
			todo!()
		}
	}
	
	#[test]
	fn test_movement_game() {
		let mut game = MovementGame::new(
			Pieces(
				Bitboard(0b1111000000000000),
				Bitboard(0b0000000000001111)
			));
	}
}

fn main() {
	println!("helo");
}
