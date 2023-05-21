extern crate enum_map;
extern crate tabled;

use crate::square::Square;
use enum_map::Enum;
use crate::board::Board;
use crate::errors::FenCharParseError;

pub mod board;
pub mod magic;
pub mod perft;
pub mod square;

mod bitboard;
mod errors;
mod masks;
mod pst;
mod zobrist;
mod history;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn loss_score(&self) -> i32 {
        match *self {
            Side::White => -50_000,
            Side::Black => 50_000
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
    pub fn value(self) -> i16 {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 300,
            Piece::Bishop => 300,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 10_000
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

    pub fn pgn_spec(self) -> String {
        match self {
            Piece::Pawn => "",
            Piece::Knight => "N",
            Piece::Bishop => "B",
            Piece::Rook => "R",
            Piece::Queen => "Q",
            Piece::King => "K"
        }.to_string()
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

    pub fn pgn_spec(&self, board: &Board) -> String {
        let piece_spec = self.piece().pgn_spec();
        let from_spec = match self.piece() {
            Piece::King => "".to_string(),
            Piece::Pawn if self.is_capture() => self.from().file_char().to_string(),
            _ => board.get_pgn_square_disambiguation(self.piece(), self.from(), self.to())
        };
        let capture_spec = if self.is_capture() { "x" } else { "" };
        let to_spec = self.to().to_string();
        format!("{}{}{}{}", piece_spec, from_spec, capture_spec, to_spec)
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

    pub fn is_pawn_involved(&self) -> bool {
        self._pieces & 0xf0 == 0x10 || self._pieces & 0x0f == 0x01
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

    pub fn pgn_spec(&self) -> String {
        let from_file_spec = self._from.file_char().to_string();
        let to_spec = self._to.to_string();
        format!("{}x{}", from_file_spec, to_spec)
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

    pub fn pgn_spec(&self) -> String {
        let to_spec = self.to().to_string();
        let promotion_spec = self.promotion_piece().pgn_spec();
        if self.is_capture() {
            let from_file_spec = self.from().file_char().to_string();
            format!("{}x{}={}", from_file_spec, to_spec, promotion_spec)
        }
        else {
            format!("{}={}", to_spec, promotion_spec)
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
    Promotion(PromotionMove)
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

    pub fn pgn_spec(&self, board: &Board) -> String {
        let spec = match self {
            Move::Regular(m) => m.pgn_spec(board),
            Move::ShortCastling(_) => "O-O".to_string(),
            Move::LongCastling(_) => "O-O-O".to_string(),
            Move::EnPassantCapture(m) => m.pgn_spec(),
            Move::Promotion(m) => m.pgn_spec()
        };
        let mut board_clone = board.clone();
        board_clone.push(self);
        if board_clone.is_in_check() {
            return if board_clone.legal_moves().is_empty() {
                format!("{}#", spec)
            } else {
                format!("{}+", spec)
            }
        }
        spec
    }

    pub fn from(&self) -> Square {
        match self {
            Move::Regular(m) => m.from(),
            Move::ShortCastling(side) => if *side == Side::White { sq!(e1) } else { sq!(e8 )},
            Move::LongCastling(side) => if *side == Side::White { sq!(e1) } else { sq!(e8 )},
            Move::EnPassantCapture(m) => m.from(),
            Move::Promotion(m) => m.from()
        }
    }

    pub fn to(&self) -> Square {
        match self {
            Move::Regular(m) => m.to(),
            Move::ShortCastling(side) => if *side == Side::White { sq!(g1) } else { sq!(g8 )},
            Move::LongCastling(side) => if *side == Side::White { sq!(c1) } else { sq!(c8 )},
            Move::EnPassantCapture(m) => m.to(),
            Move::Promotion(m) => m.to()
        }
    }

    pub fn is_capture(&self) -> bool {
        match self {
            Move::Regular(m) => m.is_capture(),
            Move::ShortCastling(_) => false,
            Move::LongCastling(_) => false,
            Move::EnPassantCapture(_) => true,
            Move::Promotion(m) => m.is_capture()
        }
    }

    pub fn piece(&self) -> Piece {
        match self {
            Move::Regular(m) => m.piece(),
            Move::ShortCastling(_) => Piece::King,
            Move::LongCastling(_) => Piece::King,
            Move::EnPassantCapture(_) => Piece::Pawn,
            Move::Promotion(_) => Piece::Pawn
        }
    }

    pub fn captured_piece(&self) -> Option<Piece> {
        match self {
            Move::Regular(m) => m.captured_piece(),
            Move::ShortCastling(_) => None,
            Move::LongCastling(_) => None,
            Move::EnPassantCapture(_) => Some(Piece::Pawn),
            Move::Promotion(m) => m.captured_piece()
        }
    }

    pub fn is_pawn_involved(&self) -> bool {
        match self {
            Move::Regular(m) => m.is_pawn_involved(),
            Move::EnPassantCapture(_) => true,
            Move::Promotion(m) => true,
            _ => false
        }
    }

    pub fn capture_value(&self) -> i16 {
        match self.captured_piece() {
            Some(p) => p.value(),
            None => 0
        }
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
            )
        }
    }
}
