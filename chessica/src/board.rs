use lazy_static::lazy_static;
use regex::Regex;
use string_builder::Builder;

use crate::bitboard::BitBoard;
use crate::magic::{build_magic_bishop_tables, build_magic_rook_tables, MagicBitBoardTable};
use crate::square::Square;
use crate::zobrist::ZobristHash;
use crate::Move::{EnPassantCapture, LongCastling, Promotion, Regular, ShortCastling};
use crate::{sq, EnPassantCaptureMove, Move, Piece, PromotionMove, RegularMove, Side};
use crate::errors::{FenParseError, IllegalMoveError};
use crate::history::History;
use crate::masks::{BLACK_PASSED_PAWN_ZONE, WHITE_PASSED_PAWN_ZONE};
use crate::pst::PstEvaluator;

#[derive(Debug, Clone, Eq, PartialEq)]
struct MoveUndoInfo {
    castling_rights: u8,
    half_move_clock: u8,
    ep_square: Option<Square>,
    is_threefold_repetition: bool,
    passed_pawns: BitBoard
}

impl MoveUndoInfo {
    fn new(castling_rights: u8, half_move_clock: u8, ep_square: Option<Square>, is_threefold_repetition: bool, passed_pawns: BitBoard) -> Self {
        MoveUndoInfo {
            castling_rights,
            half_move_clock,
            ep_square,
            is_threefold_repetition,
            passed_pawns
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Checks {
    checking_pieces: BitBoard,
    check_blocking_squares: BitBoard,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Board {
    side_to_move: Side,
    side_to_not_move: Side,
    white_pieces: BitBoard,
    black_pieces: BitBoard,
    pawns: BitBoard,
    bishops: BitBoard,
    knights: BitBoard,
    rooks: BitBoard,
    queens: BitBoard,
    kings: BitBoard,
    passed_pawns: BitBoard,
    castling_rights: u8,
    half_move_clock: u8,
    full_move_number: u16,
    ep_square: Option<Square>,
    z_hash: ZobristHash,
    pst_eval: PstEvaluator,
    move_stack: Vec<(Move, MoveUndoInfo)>,
    hash_history: History,
    is_threefold_repetition: bool
}

impl Board {
    fn back_rank(side: Side) -> u8 {
        match side {
            Side::White => 0,
            Side::Black => 7,
        }
    }

    fn pawn_double_push_rank(side: Side) -> u8 {
        match side {
            Side::White => 3,
            Side::Black => 4,
        }
    }

    fn short_castling_flag(side: Side) -> u8 {
        match side {
            Side::White => 0x1,
            Side::Black => 0x2,
        }
    }

    fn long_castling_flag(side: Side) -> u8 {
        match side {
            Side::White => 0x4,
            Side::Black => 0x8,
        }
    }

    pub fn starting_position() -> Self {
        let mut board = Board {
            side_to_move: Side::White,
            side_to_not_move: Side::Black,
            white_pieces: BitBoard::rank(0) | BitBoard::rank(1),
            black_pieces: BitBoard::rank(6) | BitBoard::rank(7),
            pawns: BitBoard::rank(1) | BitBoard::rank(6),
            knights: BitBoard::from_squares(&[sq!(b1), sq!(g1), sq!(b8), sq!(g8)]),
            bishops: BitBoard::from_squares(&[sq!(c1), sq!(f1), sq!(c8), sq!(f8)]),
            rooks: BitBoard::from_squares(&[sq!(a1), sq!(h1), sq!(a8), sq!(h8)]),
            queens: BitBoard::from_squares(&[sq!(d1), sq!(d8)]),
            kings: BitBoard::from_squares(&[sq!(e1), sq!(e8)]),
            passed_pawns: BitBoard::empty(),
            castling_rights: 0xf,
            half_move_clock: 0,
            full_move_number: 1,
            ep_square: None,
            z_hash: ZobristHash::new(),
            pst_eval: PstEvaluator::new(),
            move_stack: vec![],
            hash_history: History::new(),
            is_threefold_repetition: false
        };
        board.init_zobrist_hash();
        board.init_pst_eval();
        board.hash_history.push(board.hash());
        board
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

            let (side_to_move, side_to_not_move) = if &captures[2] == "w" {
                (Side::White, Side::Black)
            } else {
                (Side::Black, Side::White)
            };
            let castling_rights_str = &captures[3];

            let mut castling_rights: u8 = 0;
            if castling_rights_str.contains("K") {
                castling_rights |= Board::short_castling_flag(Side::White);
            }
            if castling_rights_str.contains("Q") {
                castling_rights |= Board::long_castling_flag(Side::White);
            }
            if castling_rights_str.contains("k") {
                castling_rights |= Board::short_castling_flag(Side::Black);
            }
            if castling_rights_str.contains("q") {
                castling_rights |= Board::long_castling_flag(Side::Black);
            }

            let ep_square_spec = &captures[4];
            let ep_square: Option<Square> = match ep_square_spec {
                "-" => None,
                _ => ep_square_spec.parse().ok(),
            };

            let half_move_clock: u8 = captures[5].parse().unwrap();
            let full_move_number: u16 = captures[6].parse().unwrap();

            let mut board = Board {
                side_to_move,
                side_to_not_move,
                white_pieces: BitBoard::empty(),
                black_pieces: BitBoard::empty(),
                pawns: BitBoard::empty(),
                bishops: BitBoard::empty(),
                knights: BitBoard::empty(),
                rooks: BitBoard::empty(),
                queens: BitBoard::empty(),
                kings: BitBoard::empty(),
                passed_pawns: BitBoard::empty(),
                castling_rights,
                half_move_clock,
                full_move_number,
                ep_square,
                z_hash: ZobristHash::new(),
                pst_eval: PstEvaluator::new(),
                move_stack: vec![],
                hash_history: History::new(),
                is_threefold_repetition: false
            };
            board.init_zobrist_hash();
            board.init_pst_eval();

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
                            board.add_piece(side, piece, square);
                        } else {
                            return Err(FenParseError);
                        }
                        file += 1;
                    }
                }
            }

            board.hash_history.push(board.hash());
            board.update_passed_pawns();
            return Ok(board);
        }
        Err(FenParseError)
    }

    fn init_zobrist_hash(&mut self) {
        for square in self.pawns & self.white_pieces {
            self.z_hash.flip_piece(Side::White, Piece::Pawn, square);
        }
        for square in self.bishops & self.white_pieces {
            self.z_hash.flip_piece(Side::White, Piece::Bishop, square);
        }
        for square in self.knights & self.white_pieces {
            self.z_hash.flip_piece(Side::White, Piece::Knight, square);
        }
        for square in self.rooks & self.white_pieces {
            self.z_hash.flip_piece(Side::White, Piece::Rook, square);
        }
        for square in self.queens & self.white_pieces {
            self.z_hash.flip_piece(Side::White, Piece::Queen, square);
        }
        for square in self.kings & self.white_pieces {
            self.z_hash.flip_piece(Side::White, Piece::King, square);
        }
        for square in self.pawns & self.black_pieces {
            self.z_hash.flip_piece(Side::Black, Piece::Pawn, square);
        }
        for square in self.bishops & self.black_pieces {
            self.z_hash.flip_piece(Side::Black, Piece::Bishop, square);
        }
        for square in self.knights & self.black_pieces {
            self.z_hash.flip_piece(Side::Black, Piece::Knight, square);
        }
        for square in self.rooks & self.black_pieces {
            self.z_hash.flip_piece(Side::Black, Piece::Rook, square);
        }
        for square in self.queens & self.black_pieces {
            self.z_hash.flip_piece(Side::Black, Piece::Queen, square);
        }
        for square in self.kings & self.black_pieces {
            self.z_hash.flip_piece(Side::Black, Piece::King, square);
        }
        if self.can_castle_short(Side::White) {
            self.z_hash.flip_short_castling(Side::White);
        }
        if self.can_castle_long(Side::White) {
            self.z_hash.flip_long_castling(Side::White);
        }
        if self.can_castle_short(Side::Black) {
            self.z_hash.flip_short_castling(Side::Black);
        }
        if self.can_castle_long(Side::Black) {
            self.z_hash.flip_long_castling(Side::Black);
        }
        if let Some(ep_square) = self.ep_square {
            self.z_hash.flip_ep_file(ep_square.file());
        }
        if self.side_to_move == Side::Black {
            self.z_hash.flip_black_to_move();
        }
    }

    fn init_pst_eval(&mut self) {
        for square in self.pawns & self.white_pieces {
            self.pst_eval.add_piece(Side::White, Piece::Pawn, square);
        }
        for square in self.bishops & self.white_pieces {
            self.pst_eval.add_piece(Side::White, Piece::Bishop, square);
        }
        for square in self.knights & self.white_pieces {
            self.pst_eval.add_piece(Side::White, Piece::Knight, square);
        }
        for square in self.rooks & self.white_pieces {
            self.pst_eval.add_piece(Side::White, Piece::Rook, square);
        }
        for square in self.queens & self.white_pieces {
            self.pst_eval.add_piece(Side::White, Piece::Queen, square);
        }
        for square in self.kings & self.white_pieces {
            self.pst_eval.add_piece(Side::White, Piece::King, square);
        }
        for square in self.pawns & self.black_pieces {
            self.pst_eval.add_piece(Side::Black, Piece::Pawn, square);
        }
        for square in self.bishops & self.black_pieces {
            self.pst_eval.add_piece(Side::Black, Piece::Bishop, square);
        }
        for square in self.knights & self.black_pieces {
            self.pst_eval.add_piece(Side::Black, Piece::Knight, square);
        }
        for square in self.rooks & self.black_pieces {
            self.pst_eval.add_piece(Side::Black, Piece::Rook, square);
        }
        for square in self.queens & self.black_pieces {
            self.pst_eval.add_piece(Side::Black, Piece::Queen, square);
        }
        for square in self.kings & self.black_pieces {
            self.pst_eval.add_piece(Side::Black, Piece::King, square);
        }
    }

    pub fn hash(&self) -> u64 {
        self.z_hash.value
    }

    pub fn full_move_number(&self) -> u16 {
        self.full_move_number
    }

    pub fn to_fen_string(&self) -> String {
        let mut sb = Builder::default();
        for rank in (0u8..8).rev() {
            let mut blank_square_count: u8 = 0;
            for file in 0u8..8 {
                let square = Square::from_coords(rank, file);
                if let Some(white_piece) = self.get_piece(Side::White, square) {
                    if blank_square_count > 0 {
                        sb.append(blank_square_count.to_string());
                        blank_square_count = 0;
                    }
                    sb.append(white_piece.to_fen_char(Side::White));
                } else if let Some(black_piece) = self.get_piece(Side::Black, square) {
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
        if self.castling_rights == 0 {
            sb.append("-");
        } else {
            if self.can_castle_short(Side::White) {
                sb.append("K");
            }
            if self.can_castle_long(Side::White) {
                sb.append("Q");
            }
            if self.can_castle_short(Side::Black) {
                sb.append("k");
            }
            if self.can_castle_long(Side::Black) {
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

    pub fn get_pgn_square_disambiguation(&self, piece: Piece, from: Square, to: Square) -> String {
        let legal_moves = self.legal_moves();
        let potentially_ambiguous_moves = legal_moves
            .iter()
            .filter(|&m| m.piece() == piece && m.to() == to && m.from() != from)
            .collect::<Vec<&Move>>();

        if potentially_ambiguous_moves.is_empty() {
            return "".to_string();
        }

        let mut needs_explicit_file = false;
        let mut needs_explicit_rank = false;

        if from.file() != to.file() && potentially_ambiguous_moves.iter().all(|&m| m.from().file() != from.file()) {
            needs_explicit_file = true;
        }
        else if from.rank() != to.rank() && potentially_ambiguous_moves.iter().all(|&m| m.from().rank() != from.rank()) {
            needs_explicit_rank = true;
        }
        else {
            needs_explicit_file = true;
            needs_explicit_rank = true;
        }

        let file_spec = if needs_explicit_file { from.file_char().to_string() } else { "".to_string() };
        let rank_spec = if needs_explicit_rank { from.rank_char().to_string() } else { "".to_string() };
        format!("{}{}", file_spec, rank_spec)
    }

    pub fn get_piece_count(&self, piece: Piece) -> u32 {
        match piece {
            Piece::Pawn => self.pawns.count(),
            Piece::Knight => self.knights.count(),
            Piece::Bishop => self.bishops.count(),
            Piece::Rook => self.rooks.count(),
            Piece::Queen => self.queens.count(),
            Piece::King => self.kings.count()
        }
    }

    pub fn get_pst_negamax_score(&self) -> i16 {
        self.pst_eval.score(self)
    }

    pub fn get_negamax_score(&self) -> i16 {
        self.get_material(self.side_to_move) - self.get_material(self.side_to_not_move)
    }

    pub fn get_material(&self, side: Side) -> i16 {
        let own_pieces = match side {
            Side::White => self.white_pieces,
            Side::Black => self.black_pieces
        };
        (self.pawns & own_pieces).piece_value(Piece::Pawn) +
        (self.knights & own_pieces).piece_value(Piece::Knight) +
        (self.bishops & own_pieces).piece_value(Piece::Bishop) +
        (self.rooks & own_pieces).piece_value(Piece::Rook) +
        (self.queens & own_pieces).piece_value(Piece::Queen) +
        (self.kings & own_pieces).piece_value(Piece::King)
    }

    fn get_pieces(&self, side: Side) -> BitBoard {
        match side {
            Side::White => self.white_pieces,
            Side::Black => self.black_pieces,
        }
    }

    pub fn get_piece(&self, side: Side, square: Square) -> Option<Piece> {
        let side_pieces = self.get_pieces(side);
        if !side_pieces.is_occupied(square) {
            return None;
        }
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
        } else if self.kings.is_occupied(square) {
            Some(Piece::King)
        } else {
            None
        }
    }

    fn add_piece(&mut self, side: Side, piece: Piece, square: Square) {
        match side {
            Side::White => self.white_pieces |= square,
            Side::Black => self.black_pieces |= square,
        };
        match piece {
            Piece::Pawn => self.pawns |= square,
            Piece::Bishop => self.bishops |= square,
            Piece::Knight => self.knights |= square,
            Piece::Rook => self.rooks |= square,
            Piece::Queen => self.queens |= square,
            Piece::King => self.kings |= square,
        };
        self.z_hash.flip_piece(side, piece, square);
        self.pst_eval.add_piece(side, piece, square);
    }

    fn remove_piece(&mut self, side: Side, piece: Piece, square: Square) {
        match side {
            Side::White => self.white_pieces &= !square,
            Side::Black => self.black_pieces &= !square,
        };
        match piece {
            Piece::Pawn => self.pawns &= !square,
            Piece::Bishop => self.bishops &= !square,
            Piece::Knight => self.knights &= !square,
            Piece::Rook => self.rooks &= !square,
            Piece::Queen => self.queens &= !square,
            Piece::King => self.kings &= !square,
        };
        self.z_hash.flip_piece(side, piece, square);
        self.pst_eval.remove_piece(side, piece, square);
    }

    fn apply_move(&mut self, side: Side, piece: Piece, from: Square, to: Square) {
        self.remove_piece(side, piece, from);
        self.add_piece(side, piece, to);
        match piece {
            Piece::King => {
                let flag = Board::long_castling_flag(side);
                if self.castling_rights & flag != 0 {
                    self.castling_rights &= !flag;
                    self.z_hash.flip_long_castling(side);
                }
                let flag = Board::short_castling_flag(side);
                if self.castling_rights & flag != 0 {
                    self.castling_rights &= !flag;
                    self.z_hash.flip_short_castling(side);
                }
            }
            Piece::Rook => {
                if from.rank() == Board::back_rank(side) {
                    match from.file() {
                        0 => {
                            let flag = Board::long_castling_flag(side);
                            if self.castling_rights & flag != 0 {
                                self.castling_rights &= !flag;
                                self.z_hash.flip_long_castling(side);
                            }
                        }
                        7 => {
                            let flag = Board::short_castling_flag(side);
                            if self.castling_rights & flag != 0 {
                                self.castling_rights &= !flag;
                                self.z_hash.flip_short_castling(side);
                            }
                        }
                        _ => {}
                    };
                }
            }
            _ => {}
        };
    }

    fn undo_move(&mut self, side: Side, piece: Piece, from: Square, to: Square) {
        self.remove_piece(side, piece, to);
        self.add_piece(side, piece, from);
    }

    fn apply_capture(&mut self, side: Side, piece: Piece, square: Square) {
        self.remove_piece(side, piece, square);
        if piece == Piece::Rook {
            if square.rank() == Board::back_rank(side) {
                match square.file() {
                    0 => {
                        let flag = Board::long_castling_flag(side);
                        if self.castling_rights & flag == flag {
                            self.castling_rights &= !flag;
                            self.z_hash.flip_long_castling(side);
                        }
                    }
                    7 => {
                        let flag = Board::short_castling_flag(side);
                        if self.castling_rights & flag == flag {
                            self.castling_rights &= !flag;
                            self.z_hash.flip_short_castling(side);
                        }
                    }
                    _ => {}
                };
            }
        }
    }

    fn undo_capture(&mut self, side: Side, piece: Piece, square: Square) {
        self.add_piece(side, piece, square);
    }

    fn apply_promotion(&mut self, side: Side, from: Square, to: Square, promotion: Piece) {
        self.remove_piece(side, Piece::Pawn, from);
        self.add_piece(side, promotion, to);
    }

    fn undo_promotion(&mut self, side: Side, from: Square, to: Square, promotion: Piece) {
        self.remove_piece(side, promotion, to);
        self.add_piece(side, Piece::Pawn, from);
    }

    pub fn get_uci_move(&self, uci_move: &str) -> Result<Move, IllegalMoveError> {
        let legal_moves = self.legal_moves();
        let selected_move = legal_moves.iter().find(|&m| m.to_uci_string() == uci_move);
        match selected_move {
            Some(&move_) => Ok(move_),
            None => Err(IllegalMoveError),
        }
    }

    pub fn push_uci(&mut self, uci_move: &str) -> Result<(), IllegalMoveError> {
        let legal_moves = self.legal_moves();
        let selected_move = legal_moves.iter().find(|&m| m.to_uci_string() == uci_move);
        match selected_move {
            Some(move_) => Ok(self.push(move_)),
            None => Err(IllegalMoveError),
        }
    }

    pub fn side_to_move_has_passed_pawns(&self) -> bool {
        let passed_pawns = match self.side_to_move {
            Side::White => self.white_pieces,
            Side::Black => self.black_pieces
        } & self.passed_pawns;
        passed_pawns.any()
    }

    pub fn is_passed_pawn(&self, square: Square) -> bool {
        self.passed_pawns.is_occupied(square)
    }

    pub fn get_passed_pawns(&self) -> BitBoard {
        self.passed_pawns
    }

    fn update_passed_pawns(&mut self) {
        let white_pawns = self.pawns & self.white_pieces;
        let black_pawns = self.pawns & self.black_pieces;
        let mut passed_pawns = BitBoard::empty();
        for white_pawn in white_pawns {
            let mask = BitBoard::new(WHITE_PASSED_PAWN_ZONE[white_pawn.ordinal as usize]);
            if !(black_pawns & mask).any() {
                passed_pawns |= white_pawn;
            }
        }
        for black_pawn in black_pawns {
            let mask = BitBoard::new(BLACK_PASSED_PAWN_ZONE[black_pawn.ordinal as usize]);
            if !(white_pawns & mask).any() {
                passed_pawns |= black_pawn;
            }
        }
        self.passed_pawns = passed_pawns;
    }

    pub fn push(&mut self, move_: &Move) {
        let move_undo_info =
            MoveUndoInfo::new(self.castling_rights, self.half_move_clock, self.ep_square, self.is_threefold_repetition, self.passed_pawns);
        if let Some(ep_square) = self.ep_square {
            self.z_hash.flip_ep_file(ep_square.file());
        }
        self.ep_square = None;
        match move_ {
            Regular(m) => {
                self.apply_regular_move(m);
                if m.piece() == Piece::Pawn || m.is_capture() {
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
        if move_.is_pawn_involved() {
            self.update_passed_pawns();
        }
        self.move_stack.push((move_.clone(), move_undo_info));
        std::mem::swap(&mut self.side_to_move, &mut self.side_to_not_move);
        self.z_hash.flip_black_to_move();
        if self.side_to_move == Side::White {
            self.full_move_number += 1;
        }
        let hash_count = self.hash_history.push(self.hash());
        if hash_count >= 3 {
            self.is_threefold_repetition = true;
        }
    }

    pub fn pop(&mut self) {
        if let Some((move_, move_undo_info)) = self.move_stack.pop() {
            self.hash_history.pop(self.hash());
            match move_ {
                Regular(m) => self.undo_regular_move(&m),
                ShortCastling(_side) => self.undo_short_castling_move(),
                LongCastling(_side) => self.undo_long_castling_move(),
                EnPassantCapture(m) => self.undo_ep_capture_move(&m),
                Promotion(m) => self.undo_promotion_move(&m)
            };
            let castling_rights_diff = self.castling_rights ^ move_undo_info.castling_rights;
            if castling_rights_diff != 0 {
                if castling_rights_diff & Board::short_castling_flag(Side::White) != 0 {
                    self.z_hash.flip_short_castling(Side::White);
                }
                if castling_rights_diff & Board::long_castling_flag(Side::White) != 0 {
                    self.z_hash.flip_long_castling(Side::White);
                }
                if castling_rights_diff & Board::short_castling_flag(Side::Black) != 0 {
                    self.z_hash.flip_short_castling(Side::Black);
                }
                if castling_rights_diff & Board::long_castling_flag(Side::Black) != 0 {
                    self.z_hash.flip_long_castling(Side::Black);
                }
                self.castling_rights = move_undo_info.castling_rights;
            }
            if let Some(ep_square) = self.ep_square {
                self.z_hash.flip_ep_file(ep_square.file());
            }
            self.ep_square = move_undo_info.ep_square;
            if let Some(ep_square) = self.ep_square {
                self.z_hash.flip_ep_file(ep_square.file());
            }
            self.half_move_clock = move_undo_info.half_move_clock;
            std::mem::swap(&mut self.side_to_move, &mut self.side_to_not_move);
            self.z_hash.flip_black_to_move();
            if self.side_to_move == Side::Black {
                self.full_move_number -= 1;
            }
            self.is_threefold_repetition = move_undo_info.is_threefold_repetition;
            self.passed_pawns = move_undo_info.passed_pawns;
        }
    }

    fn apply_regular_move(&mut self, m: &RegularMove) {
        if let Some(captured_piece) = m.captured_piece() {
            self.apply_capture(self.side_to_not_move, captured_piece, m.to());
        } else if m.piece() == Piece::Pawn {
            if self.side_to_move == Side::White {
                if m.from().rank() == 1 && m.to().rank() == 3 {
                    self.ep_square = m.from().delta(1, 0);
                    self.z_hash.flip_ep_file(m.to().file());
                }
            } else {
                if m.from().rank() == 6 && m.to().rank() == 4 {
                    self.ep_square = m.from().delta(-1, 0);
                    self.z_hash.flip_ep_file(m.to().file());
                }
            }
        }
        self.apply_move(self.side_to_move, m.piece(), m.from(), m.to());
    }

    fn undo_regular_move(&mut self, m: &RegularMove) {
        self.undo_move(self.side_to_not_move, m.piece(), m.from(), m.to());
        if let Some(captured_piece) = m.captured_piece() {
            self.undo_capture(self.side_to_move, captured_piece, m.to());
        }
    }

    fn apply_short_castling_move(&mut self) {
        let (king, king_to, rook, rook_to) = match self.side_to_move {
            Side::White => (sq!(e1), sq!(g1), sq!(h1), sq!(f1)),
            Side::Black => (sq!(e8), sq!(g8), sq!(h8), sq!(f8)),
        };
        self.apply_move(self.side_to_move, Piece::King, king, king_to);
        self.apply_move(self.side_to_move, Piece::Rook, rook, rook_to);
    }

    fn undo_short_castling_move(&mut self) {
        let (king, king_to, rook, rook_to) = match self.side_to_not_move {
            Side::White => (sq!(e1), sq!(g1), sq!(h1), sq!(f1)),
            Side::Black => (sq!(e8), sq!(g8), sq!(h8), sq!(f8)),
        };
        self.undo_move(self.side_to_not_move, Piece::King, king, king_to);
        self.undo_move(self.side_to_not_move, Piece::Rook, rook, rook_to);
    }

    fn apply_long_castling_move(&mut self) {
        let (king, king_to, rook, rook_to) = match self.side_to_move {
            Side::White => (sq!(e1), sq!(c1), sq!(a1), sq!(d1)),
            Side::Black => (sq!(e8), sq!(c8), sq!(a8), sq!(d8)),
        };
        self.apply_move(self.side_to_move, Piece::King, king, king_to);
        self.apply_move(self.side_to_move, Piece::Rook, rook, rook_to);
    }

    fn undo_long_castling_move(&mut self) {
        let (king, king_to, rook, rook_to) = match self.side_to_not_move {
            Side::White => (sq!(e1), sq!(c1), sq!(a1), sq!(d1)),
            Side::Black => (sq!(e8), sq!(c8), sq!(a8), sq!(d8)),
        };
        self.undo_move(self.side_to_not_move, Piece::King, king, king_to);
        self.undo_move(self.side_to_not_move, Piece::Rook, rook, rook_to);
    }

    fn apply_ep_capture_move(&mut self, m: &EnPassantCaptureMove) {
        self.apply_capture(self.side_to_not_move, Piece::Pawn, m.captured_pawn());
        self.apply_move(self.side_to_move, Piece::Pawn, m.from(), m.to());
    }

    fn undo_ep_capture_move(&mut self, m: &EnPassantCaptureMove) {
        self.undo_move(self.side_to_not_move, Piece::Pawn, m.from(), m.to());
        self.undo_capture(self.side_to_move, Piece::Pawn, m.captured_pawn());
    }

    fn apply_promotion_move(&mut self, m: &PromotionMove) {
        if let Some(captured_piece) = m.captured_piece() {
            self.apply_capture(self.side_to_not_move, captured_piece, m.to());
        }
        self.apply_promotion(self.side_to_move, m.from(), m.to(), m.promotion_piece());
    }

    fn undo_promotion_move(&mut self, m: &PromotionMove) {
        self.undo_promotion(self.side_to_not_move, m.from(), m.to(), m.promotion_piece());
        if let Some(captured_piece) = m.captured_piece() {
            self.undo_capture(self.side_to_move, captured_piece, m.to());
        }
    }

    fn can_castle_short(&self, side: Side) -> bool {
        let flag = Board::short_castling_flag(side);
        self.castling_rights & flag == flag
    }

    fn can_castle_long(&self, side: Side) -> bool {
        let flag = Board::long_castling_flag(side);
        self.castling_rights & flag == flag
    }

    pub fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    pub fn side_to_not_move(&self) -> Side {
        self.side_to_not_move
    }

    pub fn is_draw_by_threefold_repetition(&self) -> bool {
        self.is_threefold_repetition
    }

    pub fn is_draw_by_fifty_move_rule(&self) -> bool {
        self.half_move_clock >= 100
    }

    pub fn is_in_check(&self) -> bool {
        let all_pieces = self.white_pieces | self.black_pieces;
        let (own_pieces, enemy_pieces) = match self.side_to_move {
            Side::White => (self.white_pieces, self.black_pieces),
            Side::Black => (self.black_pieces, self.white_pieces),
        };

        let checks = self.checks(own_pieces, enemy_pieces, all_pieces);
        checks.checking_pieces.any()
    }

    pub fn static_exchange_score(&self, move_: Move) -> i16 {
        if !move_.is_capture() {
            return 0;
        }
        let from = move_.from();
        let to = move_.to();

        let mut all_pieces = self.white_pieces | self.black_pieces;
        let (own_pieces, enemy_pieces) = match self.side_to_move {
            Side::White => (self.white_pieces.clear(from), self.black_pieces),
            Side::Black => (self.black_pieces.clear(from), self.white_pieces),
        };

        let bishop_mask = to.bishop_moves();
        let rook_mask = to.rook_moves();

        let kings = self.kings & to.king_moves();
        let knights = self.knights & to.knight_moves();
        let bishops = self.bishops & bishop_mask;
        let rooks = self.rooks & rook_mask;
        let queens_diagonal = self.queens & bishop_mask;
        let queens_orthogonal = self.queens & rook_mask;

        let square_bb = to.bb();

        let mut own_pawns = self.pawns & own_pieces & square_bb.pawn_captures(self.side_to_not_move);
        let mut own_knights = knights & own_pieces;
        let mut own_bishops = bishops & own_pieces;
        let mut own_rooks = rooks & own_pieces;
        let mut own_queens_diagonal = queens_diagonal & own_pieces;
        let mut own_queens_orthogonal = queens_orthogonal & own_pieces;
        let mut own_king = kings & own_pieces;

        let mut enemy_pawns = self.pawns & enemy_pieces & square_bb.pawn_captures(self.side_to_move);
        let mut enemy_knights = knights & enemy_pieces;
        let mut enemy_bishops = bishops & enemy_pieces;
        let mut enemy_rooks = rooks & enemy_pieces;
        let mut enemy_queens_diagonal = queens_diagonal & enemy_pieces;
        let mut enemy_queens_orthogonal = queens_orthogonal & enemy_pieces;
        let mut enemy_king = kings & enemy_pieces;

        if own_king.any() && enemy_king.any() {
            own_king = BitBoard::empty();
            enemy_king = BitBoard::empty();
        }

        fn _pop_attacker(square: Square, attackers: &mut BitBoard, all_pieces: &mut BitBoard, blocker_mask: Option<BitBoard>) -> bool {
            let attacker = match blocker_mask {
                Some(blockers) => attackers.find(|a| (a.bounding_box(square) & *all_pieces & blockers).count() == 1),
                None => attackers.find(|_| true)
            };
            if let Some(attacker) = attacker {
                *attackers = attackers.clear(attacker);
                *all_pieces = all_pieces.clear(attacker);
            }
            attacker.is_some()
        }

        let mut score = move_.capture_value();
        let mut piece_on_square = move_.piece();
        all_pieces &= !move_.from();

        loop {
            if _pop_attacker(to, &mut enemy_pawns, &mut all_pieces, None) {
                score -= piece_on_square.value();
                piece_on_square = Piece::Pawn;
            }
            else if _pop_attacker(to, &mut enemy_knights, &mut all_pieces, None) {
                score -= piece_on_square.value();
                piece_on_square = Piece::Knight;
            }
            else if _pop_attacker(to, &mut enemy_bishops, &mut all_pieces, Some(bishop_mask)) {
                score -= piece_on_square.value();
                piece_on_square = Piece::Bishop;
            }
            else if _pop_attacker(to, &mut enemy_rooks, &mut all_pieces, Some(rook_mask)) {
                score -= piece_on_square.value();
                piece_on_square = Piece::Rook;
            }
            else if _pop_attacker(to, &mut enemy_queens_diagonal, &mut all_pieces, Some(bishop_mask)) {
                score -= piece_on_square.value();
                piece_on_square = Piece::Queen;
            }
            else if _pop_attacker(to, &mut enemy_queens_orthogonal, &mut all_pieces, Some(rook_mask)) {
                score -= piece_on_square.value();
                piece_on_square = Piece::Queen;
            }
            else if _pop_attacker(to, &mut enemy_king, &mut all_pieces, None) {
                score -= piece_on_square.value();
                piece_on_square = Piece::King;
            }
            else {
                break;
            }

            if _pop_attacker(to, &mut own_pawns, &mut all_pieces, None) {
                score += piece_on_square.value();
                piece_on_square = Piece::Pawn;
            }
            else if _pop_attacker(to, &mut own_knights, &mut all_pieces, None) {
                score += piece_on_square.value();
                piece_on_square = Piece::Knight;
            }
            else if _pop_attacker(to, &mut own_bishops, &mut all_pieces, Some(bishop_mask)) {
                score += piece_on_square.value();
                piece_on_square = Piece::Bishop;
            }
            else if _pop_attacker(to, &mut own_rooks, &mut all_pieces, Some(rook_mask)) {
                score += piece_on_square.value();
                piece_on_square = Piece::Rook;
            }
            else if _pop_attacker(to, &mut own_queens_diagonal, &mut all_pieces, Some(bishop_mask)) {
                score += piece_on_square.value();
                piece_on_square = Piece::Queen;
            }
            else if _pop_attacker(to, &mut own_queens_orthogonal, &mut all_pieces, Some(rook_mask)) {
                score += piece_on_square.value();
                piece_on_square = Piece::Queen;
            }
            else if _pop_attacker(to, &mut own_king, &mut all_pieces, None) {
                score += piece_on_square.value();
                piece_on_square = Piece::King;
            }
            else {
                break;
            }
        }

        score
    }

    pub fn legal_captures(&self) -> Vec<Move> {
        let all_moves = self.legal_moves();
        let mut captures = vec![];
        for &move_ in all_moves.iter() {
            if move_.is_capture() {
                captures.push(move_);
            }
        }
        captures
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves: Vec<Move> = vec![];
        self.legal_moves_noalloc(&mut moves);
        moves
    }

    pub fn legal_moves_noalloc(&self, moves: &mut Vec<Move>) -> usize {
        let (own_pieces, enemy_pieces) = match self.side_to_move {
            Side::White => (self.white_pieces, self.black_pieces),
            Side::Black => (self.black_pieces, self.white_pieces),
        };
        let all_pieces = own_pieces | enemy_pieces;

        let attacked_squares = self.attacked_squares(own_pieces, enemy_pieces, all_pieces);
        let checks = self.checks(own_pieces, enemy_pieces, all_pieces);
        let diagonal_pins = self.diagonal_pins(own_pieces, enemy_pieces);
        let orthogonal_pins = self.orthogonal_pins(own_pieces, enemy_pieces);
        let pins = diagonal_pins | orthogonal_pins;

        let own_king = (self.kings & own_pieces).single();
        let in_check = checks.checking_pieces.any();

        let king_moves = own_king.king_moves() & !(own_pieces | attacked_squares);
        for square in king_moves {
            let captured_piece = self.get_piece(self.side_to_not_move, square);
            moves.push(Move::regular(Piece::King, own_king, square, captured_piece));
        }
        if checks.checking_pieces.count() > 1 {
            // double check --> only king moves are legal
            return moves.len();
        }

        if !in_check {
            // castling
            let danger_squares = all_pieces | attacked_squares;
            if self.can_castle_short(self.side_to_move) {
                let king_to_square = own_king.delta(0, 2).unwrap();
                let castling_path = own_king.bounding_box(king_to_square) & !own_king;
                if !(castling_path & danger_squares).any() {
                    moves.push(Move::short_castling(self.side_to_move))
                }
            }

            if self.can_castle_long(self.side_to_move) {
                let king_to_square = own_king.delta(0, -2).unwrap();
                let extra_square = own_king.delta(0, -3).unwrap();
                let castling_path = own_king.bounding_box(king_to_square) & !own_king;
                if !(castling_path & danger_squares).any() && !all_pieces.is_occupied(extra_square)
                {
                    moves.push(Move::long_castling(self.side_to_move))
                }
            }
        }

        let check_evasion_mask = if in_check {
            checks.checking_pieces | checks.check_blocking_squares
        } else {
            BitBoard::full()
        };

        for own_bishop_or_queen in own_pieces & (self.bishops | self.queens) {
            let mut allowed_moves = self.bishop_moves(own_bishop_or_queen, all_pieces)
                & !own_pieces
                & check_evasion_mask;
            if diagonal_pins.is_occupied(own_bishop_or_queen) {
                allowed_moves &= diagonal_pins;
            } else if orthogonal_pins.is_occupied(own_bishop_or_queen) {
                allowed_moves &= (orthogonal_pins) & own_bishop_or_queen.rook_moves();
            }

            let piece = if self.bishops.is_occupied(own_bishop_or_queen) {
                Piece::Bishop
            } else {
                Piece::Queen
            };
            for square in allowed_moves {
                let captured_piece = self.get_piece(self.side_to_not_move, square);
                moves.push(Move::regular(
                    piece,
                    own_bishop_or_queen,
                    square,
                    captured_piece,
                ));
            }
        }

        for own_rook_or_queen in own_pieces & (self.rooks | self.queens) {
            let mut allowed_moves =
                self.rook_moves(own_rook_or_queen, all_pieces) & !own_pieces & check_evasion_mask;
            if diagonal_pins.is_occupied(own_rook_or_queen) {
                allowed_moves &= diagonal_pins & own_rook_or_queen.bishop_moves();
            } else if orthogonal_pins.is_occupied(own_rook_or_queen) {
                allowed_moves &= orthogonal_pins;
            }

            let piece = if self.rooks.is_occupied(own_rook_or_queen) {
                Piece::Rook
            } else {
                Piece::Queen
            };
            for square in allowed_moves {
                let captured_piece = self.get_piece(self.side_to_not_move, square);
                moves.push(Move::regular(
                    piece,
                    own_rook_or_queen,
                    square,
                    captured_piece,
                ));
            }
        }

        for own_knight in own_pieces & self.knights {
            if pins.is_occupied(own_knight) {
                // a pinned knight cannot move at all
                continue;
            }

            let allowed_knight_moves = own_knight.knight_moves() & !own_pieces & check_evasion_mask;
            for square in allowed_knight_moves {
                let captured_piece = self.get_piece(self.side_to_not_move, square);
                moves.push(Move::regular(
                    Piece::Knight,
                    own_knight,
                    square,
                    captured_piece,
                ));
            }
        }

        let own_pawns = own_pieces & self.pawns;

        let diagonally_pinned_pawns = own_pawns & diagonal_pins;
        for own_pawn in diagonally_pinned_pawns {
            let capture_mask = own_pawn.bb().pawn_captures(self.side_to_move) & enemy_pieces;
            let legal_captures = capture_mask & diagonal_pins & check_evasion_mask;
            for capture in legal_captures {
                // possible for diagonally pinned pawn to promote
                if capture.rank() == Board::back_rank(self.side_to_not_move) {
                    for promotion_piece in [Piece::Queen, Piece::Rook, Piece::Knight, Piece::Bishop]
                    {
                        let captured_piece = self.get_piece(self.side_to_not_move, capture);
                        moves.push(Move::promotion(
                            own_pawn,
                            capture,
                            promotion_piece,
                            captured_piece,
                        ));
                    }
                } else {
                    let captured_piece = self.get_piece(self.side_to_not_move, capture);
                    moves.push(Move::regular(
                        Piece::Pawn,
                        own_pawn,
                        capture,
                        captured_piece,
                    ));
                }
            }
        }

        let orthogonally_pinned_pawns = own_pawns & orthogonal_pins;
        for own_pawn in orthogonally_pinned_pawns {
            let push_mask = own_pawn.bb().pawn_pushes(self.side_to_move) & !all_pieces;
            let legal_pushes = push_mask & orthogonal_pins & check_evasion_mask;
            for push in legal_pushes {
                // impossible for orthogonally pinned pawn to promote
                moves.push(Move::regular(Piece::Pawn, own_pawn, push, None));
            }
            let double_push_rank = BitBoard::rank(Board::pawn_double_push_rank(self.side_to_move));
            let double_push_mask =
                push_mask.pawn_pushes(self.side_to_move) & double_push_rank & !all_pieces;
            let legal_double_pushes = double_push_mask & orthogonal_pins & check_evasion_mask;
            for double_push in legal_double_pushes {
                moves.push(Move::regular(Piece::Pawn, own_pawn, double_push, None));
            }
        }

        let unpinned_pawns = own_pawns & !pins;

        let pawn_pushes =
            unpinned_pawns.pawn_pushes(self.side_to_move) & !all_pieces & check_evasion_mask;
        let pawn_pushees = pawn_pushes.pawn_pushes(self.side_to_not_move);
        for (from, to) in pawn_pushees.zip(pawn_pushes) {
            if to.rank() % 7 == 0 {
                for promotion_piece in [Piece::Queen, Piece::Rook, Piece::Knight, Piece::Bishop] {
                    moves.push(Move::promotion(from, to, promotion_piece, None));
                }
            } else {
                moves.push(Move::regular(Piece::Pawn, from, to, None));
            }
        }

        let pawn_double_push_rank = BitBoard::rank(Board::pawn_double_push_rank(self.side_to_move));
        let pawn_double_pushes = (unpinned_pawns.pawn_pushes(self.side_to_move) & !all_pieces)
            .pawn_pushes(self.side_to_move)
            & pawn_double_push_rank
            & !all_pieces
            & check_evasion_mask;
        let pawn_double_pushees = pawn_double_pushes
            .pawn_pushes(self.side_to_not_move)
            .pawn_pushes(self.side_to_not_move);
        for (from, to) in pawn_double_pushees.zip(pawn_double_pushes) {
            moves.push(Move::regular(Piece::Pawn, from, to, None));
        }

        let pawn_left_captures_excl_ep = unpinned_pawns.pawn_left_captures(self.side_to_move)
            & enemy_pieces
            & check_evasion_mask;
        let pawn_left_capturers =
            pawn_left_captures_excl_ep.pawn_left_captures(self.side_to_not_move);
        let pawn_right_captures_excl_ep = unpinned_pawns.pawn_right_captures(self.side_to_move)
            & enemy_pieces
            & check_evasion_mask;
        let pawn_right_capturers =
            pawn_right_captures_excl_ep.pawn_right_captures(self.side_to_not_move);

        let pawn_captures = pawn_left_capturers
            .zip(pawn_left_captures_excl_ep)
            .chain(pawn_right_capturers.zip(pawn_right_captures_excl_ep));

        for (from, to) in pawn_captures {
            let captured_piece = self.get_piece(self.side_to_not_move, to);
            if to.rank() == Board::back_rank(self.side_to_not_move) {
                for promotion_piece in [Piece::Queen, Piece::Rook, Piece::Knight, Piece::Bishop] {
                    moves.push(Move::promotion(from, to, promotion_piece, captured_piece));
                }
            } else {
                moves.push(Move::regular(Piece::Pawn, from, to, captured_piece));
            }
        }

        if let Some(ep_square) = self.ep_square {
            let enemy_pawn = ep_square.bb().pawn_pushes(self.side_to_not_move);
            let potential_capturers = if diagonal_pins.is_occupied(ep_square) {
                unpinned_pawns | diagonally_pinned_pawns
            } else {
                unpinned_pawns
            } & ep_square.bb().pawn_captures(self.side_to_not_move);
            let own_king_rank = BitBoard::rank(own_king.rank());
            let capturers_on_own_king_rank = potential_capturers & own_king_rank;
            let mut can_capture_ep = true;
            if !(enemy_pawn & check_evasion_mask).any() {
                can_capture_ep = false;
            } else if capturers_on_own_king_rank.count() == 1 {
                // weird edge case; "partially" pinned pawn (ep capture reveals rook/queen check on same rank)
                let partial_pinners = (self.rooks | self.queens) & enemy_pieces & own_king_rank;
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
                    let captured_pawn = enemy_pawn.single();
                    moves.push(Move::en_passant(own_pawn, ep_square, captured_pawn));
                }
            }
        }

        moves.len()
    }

    fn bishop_moves(&self, square: Square, all_pieces: BitBoard) -> BitBoard {
        lazy_static! {
            static ref BISHOP_MAGICS: Vec<MagicBitBoardTable> = build_magic_bishop_tables();
        }
        match BISHOP_MAGICS.get(square.ordinal as usize) {
            Some(magic_table) => magic_table.get_moves(all_pieces),
            None => BitBoard::empty(),
        }
    }

    fn rook_moves(&self, square: Square, all_pieces: BitBoard) -> BitBoard {
        lazy_static! {
            static ref ROOK_MAGICS: Vec<MagicBitBoardTable> = build_magic_rook_tables();
        }
        match ROOK_MAGICS.get(square.ordinal as usize) {
            Some(magic_table) => magic_table.get_moves(all_pieces),
            None => BitBoard::empty(),
        }
    }

    fn attacked_squares(
        &self,
        own_pieces: BitBoard,
        enemy_pieces: BitBoard,
        all_pieces: BitBoard,
    ) -> BitBoard {
        let mut attacked_squares = (self.pawns & enemy_pieces).pawn_captures(self.side_to_not_move);
        for king in self.kings & enemy_pieces {
            attacked_squares |= king.king_moves();
        }
        for knight in self.knights & enemy_pieces {
            attacked_squares |= knight.knight_moves();
        }
        // enemy ray pieces see "through" our king
        let all_pieces_except_own_king = all_pieces & !(self.kings & own_pieces);
        for bishop_or_queen in (self.bishops | self.queens) & enemy_pieces {
            attacked_squares |= self.bishop_moves(bishop_or_queen, all_pieces_except_own_king);
        }
        for rook_or_queen in (self.rooks | self.queens) & enemy_pieces {
            attacked_squares |= self.rook_moves(rook_or_queen, all_pieces_except_own_king);
        }
        attacked_squares
    }

    fn checks(&self, own_pieces: BitBoard, enemy_pieces: BitBoard, all_pieces: BitBoard) -> Checks {
        let own_king = (self.kings & own_pieces).single();
        let enemy_pawns = self.pawns & enemy_pieces;
        let enemy_knights = self.knights & enemy_pieces;
        let enemy_bishops = self.bishops & enemy_pieces;
        let enemy_rooks = self.rooks & enemy_pieces;
        let enemy_queens = self.queens & enemy_pieces;
        let checking_pawns = enemy_pawns & own_king.bb().pawn_captures(self.side_to_move);
        let checking_knights = enemy_knights & own_king.knight_moves();
        let checking_diag_sliders =
            (enemy_bishops | enemy_queens) & self.bishop_moves(own_king, all_pieces);
        let checking_orthog_sliders =
            (enemy_rooks | enemy_queens) & self.rook_moves(own_king, all_pieces);
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

    fn diagonal_pins(&self, own_pieces: BitBoard, enemy_pieces: BitBoard) -> BitBoard {
        let own_king = (self.kings & own_pieces).single();
        let mask = own_king.bishop_moves();
        let pinners = (self.bishops | self.queens) & enemy_pieces & mask;
        let mut pin_mask = BitBoard::empty();
        for pinner in pinners {
            let pin_path = mask & pinner.bounding_box(own_king) & !own_king;
            let own_pieces_on_path = own_pieces & pin_path;
            let enemy_pieces_on_path = enemy_pieces & pin_path;
            if own_pieces_on_path.count() == 1 && enemy_pieces_on_path.count() == 1 {
                pin_mask |= pin_path;
            }
        }
        pin_mask
    }

    fn orthogonal_pins(&self, own_pieces: BitBoard, enemy_pieces: BitBoard) -> BitBoard {
        let own_king = (self.kings & own_pieces).single();
        let mask = own_king.rook_moves();
        let pinners = (self.rooks | self.queens) & enemy_pieces & mask;
        let mut pin_mask = BitBoard::empty();
        for pinner in pinners {
            let pin_path = mask & pinner.bounding_box(own_king) & !own_king;
            let own_pieces_on_path = own_pieces & pin_path;
            let enemy_pieces_on_path = enemy_pieces & pin_path;
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
    const POSITION_7: &str = "rn1qk1nr/pbppp1bp/1p4p1/4Pp2/3K4/8/PPPP1PPP/RNBQ1BNR w kq f6 0 1";
    const POSITION_8: &str = "rnb1k1nr/pppp1ppp/8/4p3/1b1P3q/2Q5/PPP1PPPP/RNB1KBNR w KQkq - 0 4";

    #[test]
    fn test_legal_moves_starting_position() {
        let board = Board::starting_position();
        let legal_moves = board.legal_moves();
        assert_eq!(legal_moves.len(), 20);
    }

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
    #[test_case(POSITION_7 ; "position 7")]
    #[test_case(POSITION_8 ; "position 8")]
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
    #[test_case(POSITION_7, 33 ; "position 7")]
    #[test_case(POSITION_8, 23 ; "position 8")]
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
    #[test_case(POSITION_7, 1, 33 ; "position 7, depth 1")]
    #[test_case(POSITION_7, 2, 983 ; "position 7, depth 2")]
    #[test_case(POSITION_7, 3, 28_964 ; "position 7, depth 3")]
    #[test_case(POSITION_7, 4, 844_341 ; "position 7, depth 4")]
    #[test_case(POSITION_8, 1, 23 ; "position 8, depth 1")]
    #[test_case(POSITION_8, 2, 1009 ; "position 8, depth 2")]
    #[test_case(POSITION_8, 3, 26_125 ; "position 8, depth 3")]
    #[test_case(POSITION_8, 4, 1_072_898 ; "position 8, depth 4")]
    fn test_perft(input_fen: &str, depth: u8, expected_result: u64) {
        let mut board = Board::parse_fen(input_fen).unwrap();
        let result = perft(&mut board, depth);
        let output_fen = board.to_fen_string();
        assert_eq!(output_fen, input_fen);
        assert_eq!(result, expected_result);
    }

    #[test_case(POSITION_1 ; "position 1")]
    #[test_case(POSITION_2 ; "position 2")]
    #[test_case(POSITION_3 ; "position 3")]
    #[test_case(POSITION_4 ; "position 4")]
    #[test_case(POSITION_5 ; "position 5")]
    #[test_case(POSITION_6 ; "position 6")]
    #[test_case(POSITION_7 ; "position 7")]
    #[test_case(POSITION_8 ; "position 8")]
    fn test_hash(input_fen: &str) {
        let mut board = Board::parse_fen(input_fen).unwrap();
        for move_ in board.legal_moves().iter() {
            let hash_before = board.hash();
            board.push(move_);
            board.pop();
            let hash_after = board.hash();
            assert_eq!(hash_before, hash_after);
        }
    }

    #[test_case("2bq1rk1/1pppbp2/r1n2np1/pB1Pp2p/Q2NP3/2P2N2/PP3PPP/R1B1K2R w KQ - 0 1", "d5c6", 300 ; "SEE 1")]
    #[test_case("2bq1rk1/1pppbp2/r1n2np1/pB1Pp2p/Q3P3/2P2N2/PP2NPPP/R1B1K2R w KQ - 0 1", "d5c6", -800 ; "SEE 2")]
    #[test_case("r1bq1rk1/ppp1ppbp/n2p1np1/4P3/2PP4/2N2N2/PP2BPPP/R1BQ1RK1 b - - 0 1", "d6e5", 0 ; "SEE 3")]
    #[test_case("r2q1rk1/pppnppbp/n5p1/4P3/2P3b1/2N2N2/PP2BPPP/R1BQ1RK1 b - - 0 1", "g7e5", 100 ; "SEE 4")]
    fn test_see(input_fen: &str, uci_move: &str, expected_score: i16) {
        let board = Board::parse_fen(input_fen).unwrap();
        let move_ = board.get_uci_move(uci_move).unwrap();
        let score = board.static_exchange_score(move_);
        assert_eq!(score, expected_score);
    }
}
