use lazy_static::lazy_static;
use regex::Regex;
use string_builder::Builder;

use crate::bitboard::BitBoard;
use crate::bitboard_magic::{
    find_bishop_magics, find_rook_magics, MagicBitBoardTable, MAGIC_INDEX_BITS,
};
use crate::square::Square;
use crate::Move::{EnPassantCapture, LongCastling, Promotion, Regular, ShortCastling};
use crate::{sq, EnPassantCaptureMove, Move, Piece, PromotionMove, RegularMove, Side};

const SHORT_CASTLING: u8 = 0x1;
const LONG_CASTLING: u8 = 0x2;

#[derive(Debug, Clone, Eq, PartialEq)]
struct MoveUndoInfo {
    castling_rights: u8,
    half_move_clock: u8,
    ep_square: Option<Square>,
}

impl MoveUndoInfo {
    fn new(
        white: &BoardSide,
        black: &BoardSide,
        half_move_clock: u8,
        ep_square: Option<Square>,
    ) -> Self {
        MoveUndoInfo {
            castling_rights: white.castling_rights | (black.castling_rights << 2),
            half_move_clock,
            ep_square,
        }
    }

    fn white_castling_rights(&self) -> u8 {
        self.castling_rights & 0x3
    }

    fn black_castling_rights(&self) -> u8 {
        self.castling_rights >> 2
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Checks {
    checking_pieces: BitBoard,
    check_blocking_squares: BitBoard,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct BoardSide {
    side: Side,
    pawns: BitBoard,
    knights: BitBoard,
    bishops: BitBoard,
    rooks: BitBoard,
    queens: BitBoard,
    king: BitBoard,
    castling_rights: u8,
}

impl BoardSide {
    fn empty(side: Side) -> Self {
        BoardSide {
            side,
            pawns: BitBoard::empty(),
            knights: BitBoard::empty(),
            bishops: BitBoard::empty(),
            rooks: BitBoard::empty(),
            queens: BitBoard::empty(),
            king: BitBoard::empty(),
            castling_rights: 0,
        }
    }

    fn starting_position(side: Side) -> Self {
        let pawn_rank = if side == Side::White { 1 } else { 6 };
        let rooks = match side {
            Side::White => [sq!(a1), sq!(h1)],
            Side::Black => [sq!(a8), sq!(h8)],
        };
        let knights = match side {
            Side::White => [sq!(b1), sq!(g1)],
            Side::Black => [sq!(b8), sq!(g8)],
        };
        let bishops = match side {
            Side::White => [sq!(c1), sq!(f1)],
            Side::Black => [sq!(c8), sq!(f8)],
        };
        let queens = match side {
            Side::White => [sq!(d1)],
            Side::Black => [sq!(d8)],
        };
        let king = match side {
            Side::White => [sq!(e1)],
            Side::Black => [sq!(e8)],
        };
        BoardSide {
            side,
            pawns: BitBoard::rank(pawn_rank),
            knights: BitBoard::empty().set_all(&knights),
            bishops: BitBoard::empty().set_all(&bishops),
            rooks: BitBoard::empty().set_all(&rooks),
            queens: BitBoard::empty().set_all(&queens),
            king: BitBoard::empty().set_all(&king),
            castling_rights: SHORT_CASTLING | LONG_CASTLING,
        }
    }

    pub fn all_pieces(&self) -> BitBoard {
        self.pawns | self.bishops | self.knights | self.rooks | self.queens | self.king
    }

    pub fn can_castle_short(&self) -> bool {
        (self.castling_rights & SHORT_CASTLING) == SHORT_CASTLING
    }

    pub fn can_castle_long(&self) -> bool {
        (self.castling_rights & LONG_CASTLING) == LONG_CASTLING
    }

    pub fn can_castle(&self) -> bool {
        self.castling_rights != 0
    }

    pub fn set_castling_rights(&mut self, castling_rights: u8) {
        self.castling_rights = castling_rights;
    }

    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        if self.pawns.is_occupied(square) {
            Some(Piece::Pawn)
        } else if self.bishops.is_occupied(square) {
            Some(Piece::Bishop)
        } else if self.knights.is_occupied(square) {
            Some(Piece::Knight)
        } else if self.rooks.is_occupied(square) {
            Some(Piece::Rook)
        } else if self.queens.is_occupied(square) {
            Some(Piece::Queen)
        } else if self.king.is_occupied(square) {
            Some(Piece::King)
        } else {
            None
        }
    }

    fn add_piece(&mut self, piece: Piece, square: Square) {
        match piece {
            Piece::Pawn => self.pawns |= square,
            Piece::Bishop => self.bishops |= square,
            Piece::Knight => self.knights |= square,
            Piece::Rook => self.rooks |= square,
            Piece::Queen => self.queens |= square,
            Piece::King => self.king |= square,
        }
    }

    fn remove_piece(&mut self, piece: Piece, square: Square) {
        match piece {
            Piece::Pawn => self.pawns &= !square,
            Piece::Bishop => self.bishops &= !square,
            Piece::Knight => self.knights &= !square,
            Piece::Rook => self.rooks &= !square,
            Piece::Queen => self.queens &= !square,
            Piece::King => self.king &= !square,
        }
    }

    fn apply_move(&mut self, piece: Piece, from: Square, to: Square) {
        self.remove_piece(piece, from);
        self.add_piece(piece, to);
        match piece {
            Piece::King => {
                self.castling_rights = 0;
            }
            Piece::Rook => {
                let starting_rank: u8 = if self.side == Side::White { 0 } else { 7 };
                if from.rank() == starting_rank {
                    match from.file() {
                        0 => {
                            self.castling_rights &= !LONG_CASTLING;
                        }
                        7 => {
                            self.castling_rights &= !SHORT_CASTLING;
                        }
                        _ => {}
                    };
                }
            }
            _ => {}
        }
    }

    fn undo_move(&mut self, piece: Piece, from: Square, to: Square) {
        self.remove_piece(piece, to);
        self.add_piece(piece, from);
    }

    fn apply_capture(&mut self, piece: Piece, square: Square) {
        self.remove_piece(piece, square);
        if piece == Piece::Rook {
            let starting_rank: u8 = if self.side == Side::White { 0 } else { 7 };
            if square.rank() == starting_rank {
                match square.file() {
                    0 => {
                        self.castling_rights &= !LONG_CASTLING;
                    }
                    7 => {
                        self.castling_rights &= !SHORT_CASTLING;
                    }
                    _ => {}
                };
            }
        }
    }

    fn undo_capture(&mut self, piece: Piece, square: Square) {
        self.add_piece(piece, square);
    }

    fn apply_promotion(&mut self, from: Square, to: Square, promotion: Piece) {
        self.remove_piece(Piece::Pawn, from);
        self.add_piece(promotion, to);
    }

    fn undo_promotion(&mut self, from: Square, to: Square, promotion: Piece) {
        self.remove_piece(promotion, to);
        self.add_piece(Piece::Pawn, from);
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Board {
    side_to_move: Side,
    white_side: BoardSide,
    black_side: BoardSide,
    half_move_clock: u8,
    full_move_number: u16,
    ep_square: Option<Square>,
    move_stack: Vec<(Move, MoveUndoInfo)>,
}

#[derive(Debug)]
pub struct FenParseError;

#[derive(Debug)]
pub struct IllegalMoveError;

impl Board {
    pub fn starting_position() -> Self {
        Board {
            side_to_move: Side::White,
            white_side: BoardSide::starting_position(Side::White),
            black_side: BoardSide::starting_position(Side::Black),
            half_move_clock: 0,
            full_move_number: 1,
            ep_square: None,
            move_stack: vec![],
        }
    }

    pub fn parse_fen(fen: &str) -> Result<Self, FenParseError> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r"([RNBQKPrnbqkp1-8/]{15,}) ([wb]) (K?Q?k?q?|-) ([a-h]?[1-8]?|-) (\d+) (\d+)"
            )
            .unwrap();
        }
        if let Some(captures) = RE.captures(fen) {
            let rows = captures[1].rsplit('/').collect::<Vec<_>>();
            if rows.len() != 8 {
                return Err(FenParseError);
            }

            let side_to_move = if &captures[2] == "w" {
                Side::White
            } else {
                Side::Black
            };
            let castling_rights = &captures[3];

            let mut white_side = BoardSide::empty(Side::White);
            let mut black_side = BoardSide::empty(Side::Black);

            if castling_rights.contains("K") {
                white_side.castling_rights |= SHORT_CASTLING;
            }
            if castling_rights.contains("Q") {
                white_side.castling_rights |= LONG_CASTLING
            }
            if castling_rights.contains("k") {
                black_side.castling_rights |= SHORT_CASTLING;
            }
            if castling_rights.contains("q") {
                black_side.castling_rights |= LONG_CASTLING
            }

            for (i, row) in rows.iter().enumerate() {
                let rank = i as u8;
                if row.len() > 8 {
                    return Err(FenParseError);
                }
                let mut file: u8 = 0;
                for c in row.bytes() {
                    if b'1' <= c && c <= b'8' {
                        file += c - b'0';
                    } else {
                        if let Ok((piece, side)) = Piece::parse_fen_char(c) {
                            let square = Square::from_coords(rank, file);
                            match side {
                                Side::White => white_side.add_piece(piece, square),
                                _ => black_side.add_piece(piece, square),
                            };
                        } else {
                            return Err(FenParseError);
                        }
                        file += 1;
                    }
                }
            }

            let ep_square_spec = &captures[4];
            let ep_square: Option<Square> = match ep_square_spec {
                "-" => None,
                _ => ep_square_spec.parse().ok(),
            };

            let half_move_clock: u8 = captures[5].parse().unwrap();
            let full_move_number: u16 = captures[6].parse().unwrap();

            let board = Board {
                side_to_move,
                white_side,
                black_side,
                half_move_clock,
                full_move_number,
                ep_square,
                move_stack: vec![],
            };

            return Ok(board);
        }
        Err(FenParseError)
    }

    pub fn to_fen_string(&self) -> String {
        let mut sb = Builder::default();
        for rank in (0u8..8).rev() {
            let mut blank_square_count: u8 = 0;
            for file in 0u8..8 {
                let square = Square::from_coords(rank, file);
                if let Some(white_piece) = self.white_side.get_piece(square) {
                    if blank_square_count > 0 {
                        sb.append(blank_square_count.to_string());
                        blank_square_count = 0;
                    }
                    sb.append(white_piece.to_fen_char(Side::White));
                } else if let Some(black_piece) = self.black_side.get_piece(square) {
                    if blank_square_count > 0 {
                        sb.append(blank_square_count.to_string());
                        blank_square_count = 0;
                    }
                    sb.append(black_piece.to_fen_char(Side::Black));
                } else {
                    blank_square_count += 1;
                }
            }
            if blank_square_count > 0 {
                sb.append(blank_square_count.to_string());
            }
            if rank > 0 {
                sb.append('/');
            }
        }

        sb.append(if self.side_to_move == Side::White {
            " w "
        } else {
            " b "
        });
        if !self.white_side.can_castle() && !self.black_side.can_castle() {
            sb.append("-");
        } else {
            if self.white_side.can_castle_short() {
                sb.append("K");
            }
            if self.white_side.can_castle_long() {
                sb.append("Q");
            }
            if self.black_side.can_castle_short() {
                sb.append("k");
            }
            if self.black_side.can_castle_long() {
                sb.append("q");
            }
        }

        sb.append(match self.ep_square {
            Some(ep) => format!(" {}", ep.to_string()),
            None => String::from(" -"),
        });

        sb.append(format!(
            " {} {}",
            self.half_move_clock, self.full_move_number
        ));

        sb.string().unwrap()
    }

    pub fn push_uci(&mut self, uci_move: &str) -> Result<(), IllegalMoveError> {
        let legal_moves = self.legal_moves();
        let selected_move = legal_moves.iter().find(|&m| m.to_uci_string() == uci_move);
        match selected_move {
            Some(move_) => Ok(self.push(move_)),
            None => Err(IllegalMoveError),
        }
    }

    pub fn push(&mut self, move_: &Move) {
        let move_undo_info = MoveUndoInfo::new(
            &self.white_side,
            &self.black_side,
            self.half_move_clock,
            self.ep_square,
        );
        self.ep_square = None;
        match move_ {
            Regular(m) => {
                self.apply_regular_move(m);
                if m.piece == Piece::Pawn || m.captured_piece.is_some() {
                    self.half_move_clock = 0;
                } else {
                    self.half_move_clock += 1;
                }
            }
            ShortCastling(_side) => {
                self.apply_short_castling_move();
                self.half_move_clock += 1;
            }
            LongCastling(_side) => {
                self.apply_long_castling_move();
                self.half_move_clock += 1;
            }
            EnPassantCapture(m) => {
                self.apply_ep_capture_move(m);
                self.half_move_clock = 0;
            }
            Promotion(m) => {
                self.apply_promotion_move(m);
                self.half_move_clock = 0;
            }
        };
        self.move_stack.push((move_.clone(), move_undo_info));
        self.side_to_move = if self.side_to_move == Side::White {
            Side::Black
        } else {
            Side::White
        };
        if self.side_to_move == Side::White {
            self.full_move_number += 1;
        }
    }

    pub fn pop(&mut self) {
        if let Some((move_, move_undo_info)) = self.move_stack.pop() {
            let prev_move_side = if self.side_to_move == Side::White {
                Side::Black
            } else {
                Side::White
            };
            match move_ {
                Regular(m) => self.undo_regular_move(&m, prev_move_side),
                ShortCastling(_side) => self.undo_short_castling_move(prev_move_side),
                LongCastling(_side) => self.undo_long_castling_move(prev_move_side),
                EnPassantCapture(m) => self.undo_ep_capture_move(&m, prev_move_side),
                Promotion(m) => self.undo_promotion_move(&m, prev_move_side),
            };
            self.white_side
                .set_castling_rights(move_undo_info.white_castling_rights());
            self.black_side
                .set_castling_rights(move_undo_info.black_castling_rights());
            self.ep_square = move_undo_info.ep_square;
            self.side_to_move = prev_move_side;
            self.half_move_clock = move_undo_info.half_move_clock;
            if self.side_to_move == Side::Black {
                self.full_move_number -= 1;
            }
        }
    }

    fn apply_regular_move(&mut self, m: &RegularMove) {
        let (own_side, enemy_side) = match self.side_to_move {
            Side::White => (&mut self.white_side, &mut self.black_side),
            Side::Black => (&mut self.black_side, &mut self.white_side),
        };
        own_side.apply_move(m.piece, m.from, m.to);
        if let Some(captured_piece) = m.captured_piece {
            enemy_side.apply_capture(captured_piece, m.to);
        } else if m.piece == Piece::Pawn {
            if own_side.side == Side::White {
                if m.from.rank() == 1 && m.to.rank() == 3 {
                    self.ep_square = m.from.delta(1, 0);
                }
            } else {
                if m.from.rank() == 6 && m.to.rank() == 4 {
                    self.ep_square = m.from.delta(-1, 0);
                }
            }
        }
    }

    fn undo_regular_move(&mut self, m: &RegularMove, move_side: Side) {
        let (own_side, enemy_side) = match move_side {
            Side::White => (&mut self.white_side, &mut self.black_side),
            Side::Black => (&mut self.black_side, &mut self.white_side),
        };
        own_side.undo_move(m.piece, m.from, m.to);
        if let Some(captured_piece) = m.captured_piece {
            enemy_side.undo_capture(captured_piece, m.to);
        }
    }

    fn apply_short_castling_move(&mut self) {
        let (own_side, king, king_to, rook, rook_to) = match self.side_to_move {
            Side::White => (&mut self.white_side, sq!(e1), sq!(g1), sq!(h1), sq!(f1)),
            Side::Black => (&mut self.black_side, sq!(e8), sq!(g8), sq!(h8), sq!(f8)),
        };
        own_side.apply_move(Piece::King, king, king_to);
        own_side.apply_move(Piece::Rook, rook, rook_to);
    }

    fn undo_short_castling_move(&mut self, move_side: Side) {
        let (own_side, king, king_to, rook, rook_to) = match move_side {
            Side::White => (&mut self.white_side, sq!(e1), sq!(g1), sq!(h1), sq!(f1)),
            Side::Black => (&mut self.black_side, sq!(e8), sq!(g8), sq!(h8), sq!(f8)),
        };
        own_side.undo_move(Piece::King, king, king_to);
        own_side.undo_move(Piece::Rook, rook, rook_to);
    }

    fn apply_long_castling_move(&mut self) {
        let (own_side, king, king_to, rook, rook_to) = match self.side_to_move {
            Side::White => (&mut self.white_side, sq!(e1), sq!(c1), sq!(a1), sq!(d1)),
            Side::Black => (&mut self.black_side, sq!(e8), sq!(c8), sq!(a8), sq!(d8)),
        };
        own_side.apply_move(Piece::King, king, king_to);
        own_side.apply_move(Piece::Rook, rook, rook_to);
    }

    fn undo_long_castling_move(&mut self, move_side: Side) {
        let (own_side, king, king_to, rook, rook_to) = match move_side {
            Side::White => (&mut self.white_side, sq!(e1), sq!(c1), sq!(a1), sq!(d1)),
            Side::Black => (&mut self.black_side, sq!(e8), sq!(c8), sq!(a8), sq!(d8)),
        };
        own_side.undo_move(Piece::King, king, king_to);
        own_side.undo_move(Piece::Rook, rook, rook_to);
    }

    fn apply_ep_capture_move(&mut self, m: &EnPassantCaptureMove) {
        let (own_side, enemy_side) = match self.side_to_move {
            Side::White => (&mut self.white_side, &mut self.black_side),
            Side::Black => (&mut self.black_side, &mut self.white_side),
        };
        own_side.apply_move(Piece::Pawn, m.from, m.to);
        enemy_side.apply_capture(Piece::Pawn, m.captured_pawn);
    }

    fn undo_ep_capture_move(&mut self, m: &EnPassantCaptureMove, move_side: Side) {
        let (own_side, enemy_side) = match move_side {
            Side::White => (&mut self.white_side, &mut self.black_side),
            Side::Black => (&mut self.black_side, &mut self.white_side),
        };
        own_side.undo_move(Piece::Pawn, m.from, m.to);
        enemy_side.undo_capture(Piece::Pawn, m.captured_pawn);
    }

    fn apply_promotion_move(&mut self, m: &PromotionMove) {
        let (own_side, enemy_side) = match self.side_to_move {
            Side::White => (&mut self.white_side, &mut self.black_side),
            Side::Black => (&mut self.black_side, &mut self.white_side),
        };
        own_side.apply_promotion(m.from, m.to, m.promotion_piece);
        if let Some(captured_piece) = m.captured_piece {
            enemy_side.apply_capture(captured_piece, m.to);
        }
    }

    fn undo_promotion_move(&mut self, m: &PromotionMove, move_side: Side) {
        let (own_side, enemy_side) = match move_side {
            Side::White => (&mut self.white_side, &mut self.black_side),
            Side::Black => (&mut self.black_side, &mut self.white_side),
        };
        own_side.undo_promotion(m.from, m.to, m.promotion_piece);
        if let Some(captured_piece) = m.captured_piece {
            enemy_side.undo_capture(captured_piece, m.to);
        }
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves: Vec<Move> = vec![];

        let own_side = self.own_side();
        let enemy_side = self.enemy_side();
        let own_pieces = own_side.all_pieces();
        let enemy_pieces = enemy_side.all_pieces();
        let all_pieces = own_pieces | enemy_pieces;

        let attacked_squares = self.attacked_squares();
        let checks = self.checks(all_pieces);
        let diagonal_pins = self.diagonal_pins();
        let orthogonal_pins = self.orthogonal_pins();
        let pins = diagonal_pins | orthogonal_pins;

        let own_king = own_side.king.single();
        let in_check = checks.checking_pieces.any();

        let king_moves = own_king.king_moves() & !(own_pieces | attacked_squares);
        for square in king_moves {
            moves.push(Regular(RegularMove {
                piece: Piece::King,
                from: own_king,
                to: square,
                captured_piece: enemy_side.get_piece(square),
            }));
        }
        if checks.checking_pieces.count() > 1 {
            // double check --> only king moves are legal
            return moves;
        }

        if !in_check {
            // castling
            let danger_squares = own_pieces | enemy_pieces | attacked_squares;
            if own_side.can_castle_short() {
                let king_to_square = own_king.delta(0, 2).unwrap();
                let castling_path = own_king.bounding_box(king_to_square) & !own_side.king;
                if !(castling_path & danger_squares).any() {
                    moves.push(ShortCastling(self.side_to_move))
                }
            }

            if own_side.can_castle_long() {
                let king_to_square = own_king.delta(0, -2).unwrap();
                let extra_square = own_king.delta(0, -3).unwrap();
                let castling_path = own_king.bounding_box(king_to_square) & !own_side.king;
                if !(castling_path & danger_squares).any() && !all_pieces.is_occupied(extra_square)
                {
                    moves.push(LongCastling(self.side_to_move))
                }
            }
        }

        let check_evasion_mask = if in_check {
            checks.checking_pieces | checks.check_blocking_squares
        } else {
            BitBoard::full()
        };

        for own_bishop_or_queen in own_side.bishops | own_side.queens {
            let mut allowed_moves = self.bishop_moves(own_bishop_or_queen, all_pieces)
                & !own_pieces
                & check_evasion_mask;
            if allowed_moves.any() {
                allowed_moves = self.bishop_moves(own_bishop_or_queen, all_pieces);
                allowed_moves &= !own_pieces & check_evasion_mask
            }
            if diagonal_pins.is_occupied(own_bishop_or_queen) {
                allowed_moves &= diagonal_pins;
            } else if orthogonal_pins.is_occupied(own_bishop_or_queen) {
                allowed_moves &= orthogonal_pins;
            }

            let piece = if own_side.bishops.is_occupied(own_bishop_or_queen) {
                Piece::Bishop
            } else {
                Piece::Queen
            };
            for square in allowed_moves {
                moves.push(Regular(RegularMove {
                    piece,
                    from: own_bishop_or_queen,
                    to: square,
                    captured_piece: enemy_side.get_piece(square),
                }));
            }
        }

        for own_rook_or_queen in own_side.rooks | own_side.queens {
            let mut allowed_moves =
                self.rook_moves(own_rook_or_queen, all_pieces) & !own_pieces & check_evasion_mask;
            if allowed_moves.any() {
                allowed_moves = self.rook_moves(own_rook_or_queen, all_pieces);
                allowed_moves &= !own_pieces & check_evasion_mask
            }
            if diagonal_pins.is_occupied(own_rook_or_queen) {
                allowed_moves &= diagonal_pins;
            } else if orthogonal_pins.is_occupied(own_rook_or_queen) {
                allowed_moves &= orthogonal_pins;
            }

            let piece = if own_side.rooks.is_occupied(own_rook_or_queen) {
                Piece::Rook
            } else {
                Piece::Queen
            };
            for square in allowed_moves {
                moves.push(Regular(RegularMove {
                    piece,
                    from: own_rook_or_queen,
                    to: square,
                    captured_piece: enemy_side.get_piece(square),
                }));
            }
        }

        for own_knight in own_side.knights {
            if pins.is_occupied(own_knight) {
                // a pinned knight cannot move at all
                continue;
            }

            let allowed_knight_moves = own_knight.knight_moves() & !own_pieces & check_evasion_mask;
            for square in allowed_knight_moves {
                moves.push(Regular(RegularMove {
                    piece: Piece::Knight,
                    from: own_knight,
                    to: square,
                    captured_piece: enemy_side.get_piece(square),
                }));
            }
        }

        let diagonally_pinned_pawns = own_side.pawns & diagonal_pins;
        for own_pawn in diagonally_pinned_pawns {
            let capture_mask =
                BitBoard::empty().set(own_pawn).pawn_captures(own_side.side) & enemy_pieces;
            let legal_captures = capture_mask & diagonal_pins & check_evasion_mask;
            for capture in legal_captures {
                // possible for diagonally pinned pawn to promote
                if capture.rank() % 7 == 0 {
                    for promotion_piece in [Piece::Queen, Piece::Rook, Piece::Knight, Piece::Bishop]
                    {
                        moves.push(Promotion(PromotionMove {
                            from: own_pawn,
                            to: capture,
                            captured_piece: enemy_side.get_piece(capture),
                            promotion_piece,
                        }));
                    }
                } else {
                    moves.push(Regular(RegularMove {
                        piece: Piece::Pawn,
                        from: own_pawn,
                        to: capture,
                        captured_piece: enemy_side.get_piece(capture),
                    }));
                }
            }
        }

        let orthogonally_pinned_pawns = own_side.pawns & orthogonal_pins;
        for own_pawn in orthogonally_pinned_pawns {
            let push_mask =
                BitBoard::empty().set(own_pawn).pawn_pushes(own_side.side) & !all_pieces;
            let legal_pushes = push_mask & orthogonal_pins & check_evasion_mask;
            for push in legal_pushes {
                // impossible for orthogonally pinned pawn to promote
                moves.push(Regular(RegularMove {
                    piece: Piece::Pawn,
                    from: own_pawn,
                    to: push,
                    captured_piece: None,
                }));
            }
            let double_push_rank = BitBoard::rank(if own_side.side == Side::White { 3 } else { 4 });
            let double_push_mask =
                push_mask.pawn_pushes(own_side.side) & double_push_rank & !all_pieces;
            let legal_double_pushes = double_push_mask & orthogonal_pins & check_evasion_mask;
            for double_push in legal_double_pushes {
                moves.push(Regular(RegularMove {
                    piece: Piece::Pawn,
                    from: own_pawn,
                    to: double_push,
                    captured_piece: None,
                }));
            }
        }

        let unpinned_pawns = own_side.pawns & !pins;

        let pawn_pushes =
            unpinned_pawns.pawn_pushes(own_side.side) & !all_pieces & check_evasion_mask;
        let pawn_pushees = pawn_pushes.pawn_pushes(enemy_side.side);
        for (from, to) in pawn_pushees.zip(pawn_pushes) {
            if to.rank() % 7 == 0 {
                for promotion_piece in [Piece::Queen, Piece::Rook, Piece::Knight, Piece::Bishop] {
                    moves.push(Promotion(PromotionMove {
                        from,
                        to,
                        captured_piece: None,
                        promotion_piece,
                    }));
                }
            } else {
                moves.push(Regular(RegularMove {
                    piece: Piece::Pawn,
                    from,
                    to,
                    captured_piece: None,
                }));
            }
        }

        let pawn_double_push_rank =
            BitBoard::rank(if own_side.side == Side::White { 3 } else { 4 });
        let pawn_double_pushes = (unpinned_pawns.pawn_pushes(own_side.side) & !all_pieces)
            .pawn_pushes(own_side.side)
            & pawn_double_push_rank
            & !all_pieces
            & check_evasion_mask;
        let pawn_double_pushees = pawn_double_pushes
            .pawn_pushes(enemy_side.side)
            .pawn_pushes(enemy_side.side);
        for (from, to) in pawn_double_pushees.zip(pawn_double_pushes) {
            moves.push(Regular(RegularMove {
                piece: Piece::Pawn,
                from,
                to,
                captured_piece: None,
            }));
        }

        let pawn_left_captures_excl_ep =
            unpinned_pawns.pawn_left_captures(own_side.side) & enemy_pieces & check_evasion_mask;
        let pawn_left_capturers = pawn_left_captures_excl_ep.pawn_left_captures(enemy_side.side);
        let pawn_right_captures_excl_ep =
            unpinned_pawns.pawn_right_captures(own_side.side) & enemy_pieces & check_evasion_mask;
        let pawn_right_capturers = pawn_right_captures_excl_ep.pawn_right_captures(enemy_side.side);

        let pawn_captures = pawn_left_capturers
            .zip(pawn_left_captures_excl_ep)
            .chain(pawn_right_capturers.zip(pawn_right_captures_excl_ep));

        for (from, to) in pawn_captures {
            let captured_piece = enemy_side.get_piece(to);
            if to.rank() % 7 == 0 {
                for promotion_piece in [Piece::Queen, Piece::Rook, Piece::Knight, Piece::Bishop] {
                    moves.push(Promotion(PromotionMove {
                        from,
                        to,
                        captured_piece,
                        promotion_piece,
                    }));
                }
            } else {
                moves.push(Regular(RegularMove {
                    piece: Piece::Pawn,
                    from,
                    to,
                    captured_piece,
                }));
            }
        }

        if let Some(ep) = self.ep_square {
            let ep_bb = BitBoard::empty().set(ep);
            let enemy_pawn = ep_bb.pawn_pushes(enemy_side.side);
            let potential_capturers = unpinned_pawns & ep_bb.pawn_captures(enemy_side.side);
            let own_king_rank = BitBoard::rank(own_king.rank());
            let capturers_on_own_king_rank = potential_capturers & own_king_rank;
            let mut can_capture_ep = true;
            if !(enemy_pawn & check_evasion_mask).any() {
                can_capture_ep = false;
            } else if capturers_on_own_king_rank.count() == 1 {
                // weird edge case; "partially" pinned pawn (ep capture reveals rook/queen check on same rank)
                let partial_pinners = (enemy_side.rooks | enemy_side.queens) & own_king_rank;
                for partial_pinner in partial_pinners {
                    let bounding_box = partial_pinner.bounding_box(own_king) & all_pieces;
                    if bounding_box.count() < 5 {
                        can_capture_ep = false;
                        break;
                    }
                }
            }
            if can_capture_ep {
                for own_pawn in potential_capturers {
                    moves.push(EnPassantCapture(EnPassantCaptureMove {
                        from: own_pawn,
                        to: ep,
                        captured_pawn: enemy_pawn.single(),
                    }));
                }
            }
        }

        moves
    }

    fn own_side(&self) -> &BoardSide {
        match self.side_to_move {
            Side::White => &self.white_side,
            Side::Black => &self.black_side,
        }
    }

    fn enemy_side(&self) -> &BoardSide {
        match self.side_to_move {
            Side::White => &self.black_side,
            Side::Black => &self.white_side,
        }
    }

    fn bishop_moves(&self, square: Square, all_pieces: BitBoard) -> BitBoard {
        lazy_static! {
            static ref BISHOP_MAGICS: Vec<MagicBitBoardTable> =
                find_bishop_magics(MAGIC_INDEX_BITS);
        }
        match BISHOP_MAGICS.get(square.ordinal as usize) {
            Some(magic_table) => magic_table.get_moves(all_pieces),
            None => BitBoard::empty(),
        }
    }

    fn rook_moves(&self, square: Square, all_pieces: BitBoard) -> BitBoard {
        lazy_static! {
            static ref ROOK_MAGICS: Vec<MagicBitBoardTable> = find_rook_magics(MAGIC_INDEX_BITS);
        }
        match ROOK_MAGICS.get(square.ordinal as usize) {
            Some(magic_table) => magic_table.get_moves(all_pieces),
            None => BitBoard::empty(),
        }
    }

    fn attacked_squares(&self) -> BitBoard {
        let own_side = self.own_side();
        let enemy_side = self.enemy_side();
        let mut attacked_squares = enemy_side.pawns.pawn_captures(enemy_side.side);
        for king in enemy_side.king {
            attacked_squares |= king.king_moves();
        }
        for knight in enemy_side.knights {
            attacked_squares |= knight.knight_moves();
        }
        // enemy ray pieces see "through" our king
        let all_pieces_except_own_king =
            (own_side.all_pieces() | enemy_side.all_pieces()) & !own_side.king;
        for bishop_or_queen in enemy_side.bishops | enemy_side.queens {
            attacked_squares |= self.bishop_moves(bishop_or_queen, all_pieces_except_own_king);
        }
        for rook_or_queen in enemy_side.rooks | enemy_side.queens {
            attacked_squares |= self.rook_moves(rook_or_queen, all_pieces_except_own_king);
        }
        attacked_squares
    }

    fn checks(&self, all_pieces: BitBoard) -> Checks {
        let own_side = self.own_side();
        let enemy_side = self.enemy_side();
        let own_king = own_side.king.single();
        let checking_pawns = enemy_side.pawns & own_side.king.pawn_captures(own_side.side);
        let checking_knights = enemy_side.knights & own_king.knight_moves();
        let checking_diag_sliders =
            (enemy_side.bishops | enemy_side.queens) & self.bishop_moves(own_king, all_pieces);
        let checking_orthog_sliders =
            (enemy_side.rooks | enemy_side.queens) & self.rook_moves(own_king, all_pieces);
        let checking_sliders = checking_diag_sliders | checking_orthog_sliders;
        let checking_pieces = checking_pawns | checking_knights | checking_sliders;
        let mut check_blocking_squares = BitBoard::empty();
        for s in checking_diag_sliders {
            check_blocking_squares |= own_king.bishop_moves() & s.bounding_box(own_king);
        }
        for s in checking_orthog_sliders {
            check_blocking_squares |= own_king.rook_moves() & s.bounding_box(own_king);
        }
        Checks {
            checking_pieces,
            check_blocking_squares,
        }
    }

    fn diagonal_pins(&self) -> BitBoard {
        let own_side = self.own_side();
        let enemy_side = self.enemy_side();
        let own_king = own_side.king.single();
        let mask = own_king.bishop_moves();
        let pinners = (enemy_side.bishops | enemy_side.queens) & mask;
        let mut pin_mask = BitBoard::empty();
        for pinner in pinners {
            let pin_path = mask & pinner.bounding_box(own_king) & !own_side.king;
            let own_pieces_on_path = own_side.all_pieces() & pin_path;
            let enemy_pieces_on_path = enemy_side.all_pieces() & pin_path;
            if own_pieces_on_path.count() == 1 && enemy_pieces_on_path.count() == 1 {
                pin_mask |= pin_path;
            }
        }
        pin_mask
    }

    fn orthogonal_pins(&self) -> BitBoard {
        let own_side = self.own_side();
        let enemy_side = self.enemy_side();
        let own_king = own_side.king.single();
        let mask = own_king.rook_moves();
        let pinners = (enemy_side.rooks | enemy_side.queens) & mask;
        let mut pin_mask = BitBoard::empty();
        for pinner in pinners {
            let pin_path = mask & pinner.bounding_box(own_king) & !own_side.king;
            let own_pieces_on_path = own_side.all_pieces() & pin_path;
            let enemy_pieces_on_path = enemy_side.all_pieces() & pin_path;
            if own_pieces_on_path.count() == 1 && enemy_pieces_on_path.count() == 1 {
                pin_mask |= pin_path;
            }
        }
        pin_mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perft::perft;
    use test_case::test_case;

    const POSITION_1: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const POSITION_2: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    const POSITION_3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    const POSITION_4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    const POSITION_5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    const POSITION_6: &str =
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

    #[test]
    fn test_parse_fen_starting_position() {
        let board1 = Board::starting_position();
        let board2 = Board::parse_fen(POSITION_1).unwrap();
        assert_eq!(board1, board2);
    }

    #[test_case(POSITION_1 ; "position 1")]
    #[test_case(POSITION_2 ; "position 2")]
    #[test_case(POSITION_3 ; "position 3")]
    #[test_case(POSITION_4 ; "position 4")]
    #[test_case(POSITION_5 ; "position 5")]
    #[test_case(POSITION_6 ; "position 6")]
    fn test_fen_round_trip(input_fen: &str) {
        let board = Board::parse_fen(input_fen).unwrap();
        let output_fen = board.to_fen_string();
        assert_eq!(output_fen.as_str(), input_fen);
    }

    #[test_case(POSITION_1, 20 ; "position 1")]
    #[test_case(POSITION_2, 48 ; "position 2")]
    #[test_case(POSITION_3, 14 ; "position 3")]
    #[test_case(POSITION_4, 6  ; "position 4")]
    #[test_case(POSITION_5, 44 ; "position 5")]
    #[test_case(POSITION_6, 46 ; "position 6")]
    fn test_legal_moves(input_fen: &str, expected_num_legal_moves: usize) {
        let board = Board::parse_fen(input_fen).unwrap();
        let legal_moves = board.legal_moves();
        assert_eq!(legal_moves.len(), expected_num_legal_moves);
    }

    #[test_case(POSITION_1, 1, 20 ; "position 1, depth 1")]
    #[test_case(POSITION_1, 2, 400 ; "position 1, depth 2")]
    #[test_case(POSITION_1, 3, 8_902 ; "position 1, depth 3")]
    #[test_case(POSITION_1, 4, 197_281 ; "position 1, depth 4")]
    #[test_case(POSITION_1, 5, 4_865_609 ; "position 1, depth 5")]
    #[test_case(POSITION_2, 1, 48 ; "position 2, depth 1")]
    #[test_case(POSITION_2, 2, 2_039 ; "position 2, depth 2")]
    #[test_case(POSITION_2, 3, 97_862 ; "position 2, depth 3")]
    #[test_case(POSITION_2, 4, 4_085_603 ; "position 2, depth 4")]
    #[test_case(POSITION_3, 1, 14 ; "position 3, depth 1")]
    #[test_case(POSITION_3, 2, 191 ; "position 3, depth 2")]
    #[test_case(POSITION_3, 3, 2_812 ; "position 3, depth 3")]
    #[test_case(POSITION_3, 4, 43_238 ; "position 3, depth 4")]
    #[test_case(POSITION_3, 5, 674_624 ; "position 3, depth 5")]
    #[test_case(POSITION_4, 1, 6  ; "position 4, depth 1")]
    #[test_case(POSITION_4, 2, 264  ; "position 4, depth 2")]
    #[test_case(POSITION_4, 3, 9_467  ; "position 4, depth 3")]
    #[test_case(POSITION_4, 4, 422_333  ; "position 4, depth 4")]
    #[test_case(POSITION_5, 1, 44 ; "position 5, depth 1")]
    #[test_case(POSITION_5, 2, 1_486 ; "position 5, depth 2")]
    #[test_case(POSITION_5, 3, 62_379 ; "position 5, depth 3")]
    #[test_case(POSITION_5, 4, 2_103_487 ; "position 5, depth 4")]
    #[test_case(POSITION_6, 1, 46 ; "position 6, depth 1")]
    #[test_case(POSITION_6, 2, 2_079 ; "position 6, depth 2")]
    #[test_case(POSITION_6, 3, 89_890 ; "position 6, depth 3")]
    #[test_case(POSITION_6, 4, 3_894_594 ; "position 6, depth 4")]
    fn test_perft(input_fen: &str, depth: u8, expected_result: u64) {
        let mut board = Board::parse_fen(input_fen).unwrap();
        let result = perft(&mut board, depth);
        let output_fen = board.to_fen_string();
        assert_eq!(output_fen, input_fen);
        assert_eq!(result, expected_result);
    }
}
