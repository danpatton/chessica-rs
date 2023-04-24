extern crate enum_map;
extern crate tabled;

use crate::square::Square;
use enum_map::Enum;
use crate::errors::FenCharParseError;

pub mod board;
pub mod magic;
pub mod perft;
pub mod square;

mod bitboard;
mod masks;
mod zobrist;
mod errors;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn loss_score(&self) -> i32 {
        match *self {
            Side::White => i32::MIN,
            Side::Black => i32::MAX
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum)]
pub enum Piece {
    Pawn = 1,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl From<u8> for Piece {
    fn from(value: u8) -> Self {
        match value {
            1 => Piece::Pawn,
            2 => Piece::Knight,
            3 => Piece::Bishop,
            4 => Piece::Rook,
            5 => Piece::Queen,
            6 => Piece::King,
            _ => panic!(),
        }
    }
}

impl Piece {
    pub fn value(self) -> i32 {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 300,
            Piece::Bishop => 300,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 1_000_000
        }
    }

    fn parse_fen_char(c: u8) -> Result<(Piece, Side), FenCharParseError> {
        match c {
            b'K' => Ok((Piece::King, Side::White)),
            b'Q' => Ok((Piece::Queen, Side::White)),
            b'R' => Ok((Piece::Rook, Side::White)),
            b'B' => Ok((Piece::Bishop, Side::White)),
            b'N' => Ok((Piece::Knight, Side::White)),
            b'P' => Ok((Piece::Pawn, Side::White)),
            b'k' => Ok((Piece::King, Side::Black)),
            b'q' => Ok((Piece::Queen, Side::Black)),
            b'r' => Ok((Piece::Rook, Side::Black)),
            b'b' => Ok((Piece::Bishop, Side::Black)),
            b'n' => Ok((Piece::Knight, Side::Black)),
            b'p' => Ok((Piece::Pawn, Side::Black)),
            _ => Err(FenCharParseError),
        }
    }

    pub fn to_fen_char(self, side: Side) -> char {
        let c = match self {
            Piece::Pawn => 'p',
            Piece::Knight => 'n',
            Piece::Bishop => 'b',
            Piece::Rook => 'r',
            Piece::Queen => 'q',
            Piece::King => 'k',
        };
        match side {
            Side::White => c.to_ascii_uppercase(),
            _ => c,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct RegularMove {
    _from: Square,
    _to: Square,
    _pieces: u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct EnPassantCaptureMove {
    _from: Square,
    _to: Square,
    _captured_pawn: Square,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PromotionMove {
    _from: Square,
    _to: Square,
    _pieces: u8,
}

impl RegularMove {
    pub fn new(piece: Piece, from: Square, to: Square, captured_piece: Option<Piece>) -> Self {
        let pieces = piece as u8
            | match captured_piece {
                Some(cp) => (cp as u8) << 4,
                None => 0,
            };
        RegularMove {
            _from: from,
            _to: to,
            _pieces: pieces,
        }
    }

    pub fn from(&self) -> Square {
        self._from
    }

    pub fn to(&self) -> Square {
        self._to
    }

    pub fn piece(&self) -> Piece {
        (self._pieces & 0x0f).into()
    }

    pub fn captured_piece(&self) -> Option<Piece> {
        match self._pieces & 0xf0 {
            0 => None,
            cp => Some((cp >> 4).into()),
        }
    }

    pub fn is_capture(&self) -> bool {
        self._pieces > 0x0f
    }
}

impl EnPassantCaptureMove {
    pub fn new(from: Square, to: Square, captured_pawn: Square) -> Self {
        EnPassantCaptureMove {
            _from: from,
            _to: to,
            _captured_pawn: captured_pawn,
        }
    }

    pub fn from(&self) -> Square {
        self._from
    }

    pub fn to(&self) -> Square {
        self._to
    }

    pub fn captured_pawn(&self) -> Square {
        self._captured_pawn
    }
}

impl PromotionMove {
    pub fn new(
        from: Square,
        to: Square,
        promotion_piece: Piece,
        captured_piece: Option<Piece>,
    ) -> Self {
        let pieces = promotion_piece as u8
            | match captured_piece {
                Some(cp) => (cp as u8) << 4,
                None => 0,
            };
        PromotionMove {
            _from: from,
            _to: to,
            _pieces: pieces,
        }
    }

    pub fn from(&self) -> Square {
        self._from
    }

    pub fn to(&self) -> Square {
        self._to
    }

    pub fn promotion_piece(&self) -> Piece {
        (self._pieces & 0x0f).into()
    }

    pub fn captured_piece(&self) -> Option<Piece> {
        match self._pieces & 0xf0 {
            0 => None,
            cp => Some((cp >> 4).into()),
        }
    }

    pub fn is_capture(&self) -> bool {
        self._pieces > 0x0f
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Move {
    Regular(RegularMove),
    ShortCastling(Side),
    LongCastling(Side),
    EnPassantCapture(EnPassantCaptureMove),
    Promotion(PromotionMove),
}

impl Move {
    pub fn regular(piece: Piece, from: Square, to: Square, captured_piece: Option<Piece>) -> Self {
        Move::Regular(RegularMove::new(piece, from, to, captured_piece))
    }

    pub fn short_castling(side: Side) -> Self {
        Move::ShortCastling(side)
    }

    pub fn long_castling(side: Side) -> Self {
        Move::LongCastling(side)
    }

    pub fn promotion(
        from: Square,
        to: Square,
        promotion_piece: Piece,
        captured_piece: Option<Piece>,
    ) -> Self {
        Move::Promotion(PromotionMove::new(
            from,
            to,
            promotion_piece,
            captured_piece,
        ))
    }

    pub fn en_passant(from: Square, to: Square, captured_pawn: Square) -> Self {
        Move::EnPassantCapture(EnPassantCaptureMove::new(from, to, captured_pawn))
    }

    pub fn to_uci_string(&self) -> String {
        match self {
            Move::Regular(m) => format!("{}{}", m.from(), m.to()),
            Move::ShortCastling(side) => {
                String::from(if *side == Side::White { "e1g1" } else { "e8g8" })
            }
            Move::LongCastling(side) => {
                String::from(if *side == Side::White { "e1c1" } else { "e8c8" })
            }
            Move::EnPassantCapture(ep) => format!("{}{}", ep.from(), ep.to()),
            Move::Promotion(m) => format!(
                "{}{}{}",
                m.from(),
                m.to(),
                m.promotion_piece().to_fen_char(Side::Black)
            ),
        }
    }
}
