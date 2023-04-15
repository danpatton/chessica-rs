extern crate enum_map;
extern crate tabled;

use enum_map::Enum;
use crate::square::Square;

pub mod board;
pub mod perft;

mod bitboard;
mod bitboard_magic;
mod bitboard_masks;
mod square;
mod zobrist;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum)]
pub enum Side {
    White,
    Black,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

pub struct FenCharParseError;

impl Piece {
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegularMove {
    pub piece: Piece,
    pub from: Square,
    pub to: Square,
    pub captured_piece: Option<Piece>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EnPassantCaptureMove {
    pub from: Square,
    pub to: Square,
    pub captured_pawn: Square,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PromotionMove {
    pub from: Square,
    pub to: Square,
    pub promotion_piece: Piece,
    pub captured_piece: Option<Piece>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Move {
    Regular(RegularMove),
    ShortCastling(Side),
    LongCastling(Side),
    EnPassantCapture(EnPassantCaptureMove),
    Promotion(PromotionMove),
}

impl Move {
    pub fn to_uci_string(&self) -> String {
        match self {
            Move::Regular(m) => format!("{}{}", m.from, m.to),
            Move::ShortCastling(side) => {
                String::from(if *side == Side::White { "e1g1" } else { "e8g8" })
            }
            Move::LongCastling(side) => {
                String::from(if *side == Side::White { "e1c1" } else { "e8c8" })
            }
            Move::EnPassantCapture(ep) => format!("{}{}", ep.from, ep.to),
            Move::Promotion(m) => format!(
                "{}{}{}",
                m.from,
                m.to,
                m.promotion_piece.to_fen_char(Side::Black)
            ),
        }
    }
}
