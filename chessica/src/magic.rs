use crate::bitboard::BitBoard;
use crate::square::Square;
use rand::Rng;

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

pub fn find_fancy_rook_magics(min_index_bits: u8, max_attempts: u32) -> Vec<MagicBitBoardTable> {
    find_fancy_magics(min_index_bits, max_attempts, &ROOK)
}

pub fn find_fancy_bishop_magics(min_index_bits: u8, max_attempts: u32) -> Vec<MagicBitBoardTable> {
    find_fancy_magics(min_index_bits, max_attempts, &BISHOP)
}

fn find_fancy_magics(
    min_index_bits: u8,
    max_attempts: u32,
    slider: &Slider,
) -> Vec<MagicBitBoardTable> {
    let mut tables: Vec<MagicBitBoardTable> = vec![];
    for ordinal in 0u8..64 {
        let square = Square::from_ordinal(ordinal);
        let mut index_bits = min_index_bits;
        loop {
            if let Ok(table) = try_find_magic_table(slider, square, index_bits, max_attempts) {
                // println!("Found {}-bit magic {:#018x} for {}", index_bits, table.magic, square);
                println!("   ({}, {:#018x}),", index_bits, table.magic);
                tables.push(table);
                break;
            }
            index_bits += 1;
        }
    }
    tables
}

fn build_magic_tables(slider: &Slider, hardcoded_magics: &[(u8, u64)]) -> Vec<MagicBitBoardTable> {
    let mut tables: Vec<MagicBitBoardTable> = vec![];
    for (i, &(index_bits, magic)) in hardcoded_magics.iter().enumerate() {
        let index_shift = 64 - index_bits;
        let square = Square::from_ordinal(i as u8);
        let blocker_mask = slider.blocker_mask(square);
        let table = try_make_magic_table(slider, square, blocker_mask, index_bits, magic).unwrap();
        tables.push(MagicBitBoardTable {
            index_shift,
            blocker_mask,
            magic,
            table,
        });
    }
    tables
}

pub fn build_magic_rook_tables() -> Vec<MagicBitBoardTable> {
    build_magic_tables(&ROOK, &ROOK_MAGICS)
}

pub fn build_magic_bishop_tables() -> Vec<MagicBitBoardTable> {
    build_magic_tables(&BISHOP, &BISHOP_MAGICS)
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

#[derive(Debug)]
struct MagicTableMaxAttemptsExceededError;

fn try_find_magic_table(
    slider: &Slider,
    square: Square,
    index_bits: u8,
    max_attempts: u32,
) -> Result<MagicBitBoardTable, MagicTableMaxAttemptsExceededError> {
    let index_shift = 64 - index_bits;
    let blocker_mask = slider.blocker_mask(square);
    let mut rng = rand::thread_rng();
    let mut attempts = 0;
    loop {
        let magic = rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>();
        if let Ok(table) = try_make_magic_table(slider, square, blocker_mask, index_bits, magic) {
            return Ok(MagicBitBoardTable {
                index_shift,
                blocker_mask,
                magic,
                table,
            });
        }
        attempts += 1;
        if attempts >= max_attempts {
            break;
        }
    }
    Err(MagicTableMaxAttemptsExceededError)
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

pub const ROOK_MAGICS: [(u8, u64); 64] = [
    (12, 0x00800080c0009020),
    (11, 0x0140002000100044),
    (11, 0x82001009a20080c0),
    (11, 0x0100090024203000),
    (11, 0x06802a0400804800),
    (11, 0x0080040080290200),
    (11, 0x0400100a00c80314),
    (12, 0x8180004080002100),
    (11, 0x2101800840009022),
    (10, 0x0082002302014080),
    (10, 0x0000805000802000),
    (10, 0x0820802800805002),
    (10, 0x0022808008000400),
    (10, 0x0086000a00143128),
    (10, 0x0906000200480401),
    (11, 0x0011000082522900),
    (11, 0x080084800040002a),
    (10, 0xc004c04010022000),
    (10, 0x0005090020004010),
    (10, 0x0808808018001004),
    (10, 0xa008008004008008),
    (10, 0x0016c80104201040),
    (10, 0x9200808002002100),
    (11, 0x80020200040088e1),
    (11, 0x0012209080084000),
    (10, 0x0002010200208040),
    (10, 0x2804430100352000),
    (10, 0x0000080080100080),
    (10, 0x80000800800c0080),
    (10, 0x1002200801044030),
    (10, 0x8202000200084104),
    (11, 0x00410001000c8242),
    (11, 0x8004c00180800120),
    (10, 0x1002400081002100),
    (10, 0x0a00809006802000),
    (10, 0xa020100080800800),
    (10, 0x0200810400800800),
    (10, 0x5040902028014004),
    (10, 0x080021b024000208),
    (11, 0xc04000410200008c),
    (11, 0x0030400020808000),
    (10, 0x1011482010014001),
    (10, 0x4009100020008080),
    (10, 0x0402011220420008),
    (10, 0x9000080004008080),
    (10, 0x8006001810220044),
    (10, 0x0400309822040001),
    (11, 0x40a4040481460001),
    (11, 0x0000800020400080),
    (10, 0x0040401000200040),
    (10, 0x4003002000144100),
    (10, 0x0122004110204a00),
    (10, 0x0406000520100a00),
    (10, 0x8100040080020080),
    (10, 0x011818100a030400),
    (11, 0x2031104400810200),
    (12, 0x0880088103204011),
    (11, 0x0020a81100814003),
    (11, 0x0100c3004850a001),
    (11, 0x0104200509001001),
    (11, 0x1062001020883482),
    (11, 0x0061000608240001),
    (11, 0x0200020810250084),
    (12, 0x010001004400208e),
];

pub const BISHOP_MAGICS: [(u8, u64); 64] = [
    (6, 0xa020410208210220),
    (5, 0x0220040400822000),
    (5, 0x8184811202030000),
    (5, 0x8110990202010902),
    (5, 0x002c04e018001122),
    (5, 0x006504a004044001),
    (5, 0x0002061096081014),
    (6, 0x0420440605032000),
    (5, 0x01080c980a141c00),
    (5, 0x0400200a02204300),
    (5, 0x4200101084810310),
    (5, 0x0200590401002100),
    (5, 0x84020110c042010d),
    (5, 0x00031c2420880088),
    (5, 0x10002104110440a0),
    (5, 0x0000010582104240),
    (5, 0x00080d501010009c),
    (5, 0x4092000408080100),
    (7, 0x0001001828010010),
    (7, 0x40220220208030a0),
    (7, 0x8201008090400000),
    (7, 0x0000202202012008),
    (5, 0x0008400404020810),
    (5, 0x0042004082088202),
    (5, 0x007a080110101000),
    (5, 0x6094101002028800),
    (7, 0x0018080004004410),
    (9, 0x688200828800810a),
    (9, 0x0881004409004004),
    (7, 0x1051020009004144),
    (5, 0x0202008102080100),
    (5, 0x0401010000484800),
    (5, 0x4001300800302100),
    (5, 0x50240c0420204926),
    (7, 0x0008640100102102),
    (9, 0x4800100821040400),
    (9, 0x00200240400400b0),
    (7, 0x0008030100027004),
    (5, 0x2001080200a48242),
    (5, 0x000400aa02002100),
    (5, 0x0a82501004000820),
    (5, 0x0002480211282840),
    (7, 0x0081001802001400),
    (7, 0x4008014010400203),
    (7, 0x0000080900410400),
    (7, 0x0220210301080200),
    (5, 0x00200b0401010080),
    (5, 0x0301012408890100),
    (5, 0x2202015016101444),
    (5, 0x0801008084210000),
    (5, 0x0a20051480900032),
    (5, 0x0000400042120880),
    (5, 0x000006100e020000),
    (5, 0x0600083004082800),
    (5, 0x2c88501312140010),
    (5, 0x0804080200420000),
    (6, 0x0040802090042000),
    (5, 0x4020006486107088),
    (5, 0x0008801052080400),
    (5, 0x631000244420a802),
    (5, 0x0080400820204100),
    (5, 0x101000100c100420),
    (5, 0x011040044840c100),
    (6, 0x0040080104012242),
];
