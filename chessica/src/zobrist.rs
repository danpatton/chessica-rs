use enum_map::{enum_map, EnumMap};
use lazy_static::lazy_static;
use rand::prelude::ThreadRng;
use rand::Rng;
use crate::{Piece, Side};
use crate::square::Square;

#[derive(Debug, Eq, PartialEq)]
pub struct ZobristHashKeys {
    pub black_to_move_key: u64,
    pub piece_keys: EnumMap<Side, EnumMap<Piece, Vec<u64>>>,
    pub short_castling_keys: EnumMap<Side, u64>,
    pub long_castling_keys: EnumMap<Side, u64>,
    pub en_passant_file_keys: Vec<u64>,
}

impl ZobristHashKeys {
    fn generate_piece_keys(rng: &mut ThreadRng) -> EnumMap<Piece, Vec<u64>> {
        enum_map! {
            Piece::Pawn => (0u8..64).map(|_| rng.gen::<u64>()).collect(),
            Piece::Bishop => (0u8..64).map(|_| rng.gen::<u64>()).collect(),
            Piece::Knight => (0u8..64).map(|_| rng.gen::<u64>()).collect(),
            Piece::Rook => (0u8..64).map(|_| rng.gen::<u64>()).collect(),
            Piece::Queen => (0u8..64).map(|_| rng.gen::<u64>()).collect(),
            Piece::King => (0u8..64).map(|_| rng.gen::<u64>()).collect(),
        }
    }

    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        ZobristHashKeys {
            black_to_move_key: rng.gen::<u64>(),
            piece_keys: enum_map! {
                Side::White => ZobristHashKeys::generate_piece_keys(&mut rng),
                Side::Black => ZobristHashKeys::generate_piece_keys(&mut rng),
            },
            short_castling_keys: enum_map! {
                Side::White => rng.gen::<u64>(),
                Side::Black => rng.gen::<u64>(),
            },
            long_castling_keys: enum_map! {
                Side::White => rng.gen::<u64>(),
                Side::Black => rng.gen::<u64>(),
            },
            en_passant_file_keys: (0..8).map(|_| rng.gen::<u64>()).collect()
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ZobristHash {
    pub value: u64,
    keys: &'static ZobristHashKeys
}

impl ZobristHash {
    pub fn new() -> Self {
        lazy_static! {
            static ref KEYS: ZobristHashKeys = ZobristHashKeys::generate();
        }
        ZobristHash {
            value: 0,
            keys: &KEYS
        }
    }

    pub fn flip_black_to_move(&mut self) {
        self.value ^= self.keys.black_to_move_key;
    }

    pub fn flip_piece(&mut self, side: Side, piece: Piece, square: Square) {
        self.value ^= self.keys.piece_keys[side][piece][square.ordinal as usize];
    }

    pub fn flip_ep_file(&mut self, file: u8) {
        self.value ^= self.keys.en_passant_file_keys[file as usize];
    }

    pub fn flip_long_castling(&mut self, side: Side) {
        self.value ^= self.keys.long_castling_keys[side];
    }

    pub fn flip_short_castling(&mut self, side: Side) {
        self.value ^= self.keys.short_castling_keys[side];
    }
}
