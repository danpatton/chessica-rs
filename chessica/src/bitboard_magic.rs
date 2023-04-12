use crate::bitboard::BitBoard;
use crate::square::Square;
use rand::Rng;

pub const MAGIC_INDEX_BITS: u8 = 14;

#[derive(Debug)]
pub struct MagicBitBoardTable {
    index_shift: u8,
    blocker_mask: BitBoard,
    magic: u64,
    table: Vec<BitBoard>,
}

impl MagicBitBoardTable {
    pub fn get_moves(&self, all_pieces: BitBoard) -> BitBoard {
        let blockers = all_pieces & self.blocker_mask;
        let index = blockers.magic_hash_index(self.magic, self.index_shift);
        *self.table.get(index).unwrap()
    }
}

pub fn find_rook_magics(index_bits: u8) -> Vec<MagicBitBoardTable> {
    let mut tables: Vec<MagicBitBoardTable> = vec![];
    for ordinal in 0u8..64 {
        let square = Square::from_ordinal(ordinal);
        let table = find_magic_table(&ROOK, square, index_bits);
        tables.push(table);
    }
    tables
}

pub fn find_bishop_magics(index_bits: u8) -> Vec<MagicBitBoardTable> {
    let mut tables: Vec<MagicBitBoardTable> = vec![];
    for ordinal in 0u8..64 {
        let square = Square::from_ordinal(ordinal);
        let table = find_magic_table(&BISHOP, square, index_bits);
        tables.push(table);
    }
    tables
}

struct Slider {
    deltas: [[i8; 2]; 4],
}

impl Slider {
    pub fn moves(&self, square: Square, blockers: BitBoard) -> BitBoard {
        let mut moves = BitBoard::empty();
        for deltas in self.deltas {
            let rank_delta = deltas[0];
            let file_delta = deltas[1];
            let mut current = square;
            while let Some(next) = current.delta(rank_delta, file_delta) {
                current = next;
                moves = moves.set(current);
                if blockers.is_occupied(current) {
                    break;
                }
            }
        }
        moves
    }

    pub fn blocker_mask(&self, square: Square) -> BitBoard {
        let mut mask = self.moves(square, BitBoard::empty());
        if square.rank() != 0 {
            mask &= !BitBoard::rank(0);
        }
        if square.rank() != 7 {
            mask &= !BitBoard::rank(7);
        }
        if square.file() != 0 {
            mask &= !BitBoard::file(0);
        }
        if square.file() != 7 {
            mask &= !BitBoard::file(7);
        }
        mask
    }
}

const ROOK: Slider = Slider {
    deltas: [[0, 1], [1, 0], [0, -1], [-1, 0]],
};

const BISHOP: Slider = Slider {
    deltas: [[-1, -1], [-1, 1], [1, -1], [1, 1]],
};

fn find_magic_table(slider: &Slider, square: Square, index_bits: u8) -> MagicBitBoardTable {
    let index_shift = 64 - index_bits;
    let blocker_mask = slider.blocker_mask(square);
    let mut rng = rand::thread_rng();
    loop {
        let magic = rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>();
        if let Ok(table) = try_make_magic_table(slider, square, blocker_mask, index_bits, magic) {
            return MagicBitBoardTable {
                index_shift,
                blocker_mask,
                magic,
                table,
            };
        }
    }
}

#[derive(Debug)]
struct HashCollisionError;

fn try_make_magic_table(
    slider: &Slider,
    square: Square,
    blocker_mask: BitBoard,
    index_bits: u8,
    magic: u64,
) -> Result<Vec<BitBoard>, HashCollisionError> {
    let index_shift = 64 - index_bits;
    let table_size = 1 << index_bits;
    let mut table = vec![BitBoard::empty(); table_size];

    for blockers in blocker_mask.subsets() {
        let moves = slider.moves(square, blockers);
        let index = blockers.magic_hash_index(magic, index_shift);
        let table_entry = &mut table[index];
        if table_entry.is_empty() {
            *table_entry = moves;
        } else if *table_entry != moves {
            // non-constructive collision --> this is not the magic we're looking for
            return Err(HashCollisionError);
        }
    }

    Ok(table)
}
