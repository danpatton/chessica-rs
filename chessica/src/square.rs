use crate::bitboard::BitBoard;
use crate::masks::{BISHOPS_MOVE, BOUNDING_BOX, KINGS_MOVE, KNIGHTS_MOVE, ROOKS_MOVE};
use std::fmt;
use std::fmt::Formatter;
use std::ops;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Square {
    pub ordinal: u8,
}

impl Square {
    pub fn from_coords(rank: u8, file: u8) -> Self {
        Square {
            ordinal: rank * 8 + file,
        }
    }

    pub fn from_ordinal(ordinal: u8) -> Self {
        Square { ordinal }
    }

    pub fn bb(self) -> BitBoard {
        BitBoard { value: self.bit() }
    }

    pub fn bit(self) -> u64 {
        1 << self.ordinal
    }

    pub fn rank(self) -> u8 {
        self.ordinal / 8
    }

    pub fn file(self) -> u8 {
        self.ordinal % 8
    }

    fn rank_char(self) -> char {
        (b'1' + self.rank()) as char
    }

    fn file_char(self) -> char {
        (b'a' + self.file()) as char
    }

    pub fn delta(self, rank_delta: i8, file_delta: i8) -> Option<Square> {
        let new_rank = self.rank() as i8 + rank_delta;
        let new_file = self.file() as i8 + file_delta;
        let off_board = new_rank < 0 || new_rank > 7 || new_file < 0 || new_file > 7;
        if off_board {
            None
        } else {
            Some(Square::from_coords(new_rank as u8, new_file as u8))
        }
    }

    pub fn knight_moves(self) -> BitBoard {
        BitBoard::new(KNIGHTS_MOVE[self.ordinal as usize])
    }

    pub fn bishop_moves(self) -> BitBoard {
        BitBoard::new(BISHOPS_MOVE[self.ordinal as usize])
    }

    pub fn rook_moves(self) -> BitBoard {
        BitBoard::new(ROOKS_MOVE[self.ordinal as usize])
    }

    pub fn queen_moves(self) -> BitBoard {
        BitBoard::new(BISHOPS_MOVE[self.ordinal as usize] | ROOKS_MOVE[self.ordinal as usize])
    }

    pub fn king_moves(self) -> BitBoard {
        BitBoard::new(KINGS_MOVE[self.ordinal as usize])
    }

    pub fn bounding_box(self, other: Square) -> BitBoard {
        BitBoard::new(BOUNDING_BOX[self.ordinal as usize][other.ordinal as usize])
    }
}

impl ops::Not for Square {
    type Output = BitBoard;

    fn not(self) -> Self::Output {
        BitBoard { value: !self.bit() }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseSquareError;

impl FromStr for Square {
    type Err = ParseSquareError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();
        if s.len() == 2 {
            let file = b.get(0).unwrap();
            let rank = b.get(1).unwrap();
            if *file >= b'a' && *file <= b'h' && *rank >= b'1' && *rank <= b'8' {
                return Ok(Square::from_coords(*rank - b'1', *file - b'a'));
            }
        }
        Err(ParseSquareError)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let chars = vec![self.file_char(), self.rank_char()];
        let s: String = chars.into_iter().collect();
        f.write_str(&s)
    }
}

#[macro_export]
macro_rules! sq {
    (a1) => {
        Square::from_ordinal(0)
    };
    (b1) => {
        Square::from_ordinal(1)
    };
    (c1) => {
        Square::from_ordinal(2)
    };
    (d1) => {
        Square::from_ordinal(3)
    };
    (e1) => {
        Square::from_ordinal(4)
    };
    (f1) => {
        Square::from_ordinal(5)
    };
    (g1) => {
        Square::from_ordinal(6)
    };
    (h1) => {
        Square::from_ordinal(7)
    };
    (a2) => {
        Square::from_ordinal(8)
    };
    (b2) => {
        Square::from_ordinal(9)
    };
    (c2) => {
        Square::from_ordinal(10)
    };
    (d2) => {
        Square::from_ordinal(11)
    };
    (e2) => {
        Square::from_ordinal(12)
    };
    (f2) => {
        Square::from_ordinal(13)
    };
    (g2) => {
        Square::from_ordinal(14)
    };
    (h2) => {
        Square::from_ordinal(15)
    };
    (a3) => {
        Square::from_ordinal(16)
    };
    (b3) => {
        Square::from_ordinal(17)
    };
    (c3) => {
        Square::from_ordinal(18)
    };
    (d3) => {
        Square::from_ordinal(19)
    };
    (e3) => {
        Square::from_ordinal(20)
    };
    (f3) => {
        Square::from_ordinal(21)
    };
    (g3) => {
        Square::from_ordinal(22)
    };
    (h3) => {
        Square::from_ordinal(23)
    };
    (a4) => {
        Square::from_ordinal(24)
    };
    (b4) => {
        Square::from_ordinal(25)
    };
    (c4) => {
        Square::from_ordinal(26)
    };
    (d4) => {
        Square::from_ordinal(27)
    };
    (e4) => {
        Square::from_ordinal(28)
    };
    (f4) => {
        Square::from_ordinal(29)
    };
    (g4) => {
        Square::from_ordinal(30)
    };
    (h4) => {
        Square::from_ordinal(31)
    };
    (a5) => {
        Square::from_ordinal(32)
    };
    (b5) => {
        Square::from_ordinal(33)
    };
    (c5) => {
        Square::from_ordinal(34)
    };
    (d5) => {
        Square::from_ordinal(35)
    };
    (e5) => {
        Square::from_ordinal(36)
    };
    (f5) => {
        Square::from_ordinal(37)
    };
    (g5) => {
        Square::from_ordinal(38)
    };
    (h5) => {
        Square::from_ordinal(39)
    };
    (a6) => {
        Square::from_ordinal(40)
    };
    (b6) => {
        Square::from_ordinal(41)
    };
    (c6) => {
        Square::from_ordinal(42)
    };
    (d6) => {
        Square::from_ordinal(43)
    };
    (e6) => {
        Square::from_ordinal(44)
    };
    (f6) => {
        Square::from_ordinal(45)
    };
    (g6) => {
        Square::from_ordinal(46)
    };
    (h6) => {
        Square::from_ordinal(47)
    };
    (a7) => {
        Square::from_ordinal(48)
    };
    (b7) => {
        Square::from_ordinal(49)
    };
    (c7) => {
        Square::from_ordinal(50)
    };
    (d7) => {
        Square::from_ordinal(51)
    };
    (e7) => {
        Square::from_ordinal(52)
    };
    (f7) => {
        Square::from_ordinal(53)
    };
    (g7) => {
        Square::from_ordinal(54)
    };
    (h7) => {
        Square::from_ordinal(55)
    };
    (a8) => {
        Square::from_ordinal(56)
    };
    (b8) => {
        Square::from_ordinal(57)
    };
    (c8) => {
        Square::from_ordinal(58)
    };
    (d8) => {
        Square::from_ordinal(59)
    };
    (e8) => {
        Square::from_ordinal(60)
    };
    (f8) => {
        Square::from_ordinal(61)
    };
    (g8) => {
        Square::from_ordinal(62)
    };
    (h8) => {
        Square::from_ordinal(63)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accessors() {
        let e4 = Square::from_ordinal(28);
        assert_eq!(e4.rank(), 3);
        assert_eq!(e4.file(), 4);
    }

    #[test]
    fn test_sq_macro() {
        let e4 = sq!(e4);
        assert_eq!(e4.rank(), 3);
        assert_eq!(e4.file(), 4);
        let h5 = sq!(h5);
        assert_eq!(h5.rank(), 4);
        assert_eq!(h5.file(), 7);
    }

    #[test]
    fn test_parse() {
        let e4: Square = "e4".parse().unwrap();
        assert_eq!(e4.rank(), 3);
        assert_eq!(e4.file(), 4);
        let h5: Square = "h5".parse().unwrap();
        assert_eq!(h5.rank(), 4);
        assert_eq!(h5.file(), 7);
    }

    #[test]
    fn test_parse_invalid() {
        let err1: Result<Square, ParseSquareError> = "e9".parse();
        assert_eq!(err1.is_err(), true);
        let err2: Result<Square, ParseSquareError> = "j4".parse();
        assert_eq!(err2.is_err(), true);
        let err3: Result<Square, ParseSquareError> = "e4g".parse();
        assert_eq!(err3.is_err(), true);
    }

    #[test]
    fn test_to_string() {
        let e4 = Square::from_ordinal(28);
        assert_eq!(e4.to_string(), "e4");
    }
}
