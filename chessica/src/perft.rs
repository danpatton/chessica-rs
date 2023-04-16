use crate::board::Board;

#[derive(Clone, Copy)]
pub struct PerftHashEntry(pub u64, pub u64);

pub fn perft(board: &mut Board, depth: u8) -> u64 {
    match depth {
        0 => 1,
        1 => board.legal_moves().len() as u64,
        _ => {
            let legal_moves = board.legal_moves();
            let mut count: u64 = 0;
            for move_ in legal_moves.iter() {
                board.push(move_);
                count += perft(board, depth - 1);
                board.pop();
            }
            count
        }
    }
}

pub fn perft_h(board: &mut Board, depth: u8, hash_table: &mut Vec<PerftHashEntry>) -> u64 {
    assert!(depth <= 7);
    assert!(hash_table.len().is_power_of_two());
    assert!(hash_table.len() >= 16);
    let depth_shift = (hash_table.len().trailing_zeros() - 3) as u8;
    let hash_mask = (1u64 << depth_shift) - 1;

    fn _impl(board: &mut Board, depth: u64, depth_shift: u8, hash_mask: u64, hash_table: &mut Vec<PerftHashEntry>) -> u64 {
        let hash = board.hash();
        let idx = ((depth << depth_shift) | (hash & hash_mask)) as usize;
        let entry = &hash_table[idx];
        if entry.0 == hash {
            return entry.1;
        }
        let result = match depth {
            0 => 1,
            1 => board.legal_moves().len() as u64,
            _ => {
                let legal_moves = board.legal_moves();
                let mut count: u64 = 0;
                for move_ in legal_moves.iter() {
                    board.push(move_);
                    count += _impl(board, depth - 1, depth_shift, hash_mask, hash_table);
                    board.pop();
                }
                count
            }
        };
        let entry = &mut hash_table[idx];
        *entry = PerftHashEntry(hash, result);
        result
    }

    _impl(board, depth as u64, depth_shift, hash_mask, hash_table)
}

pub fn perft_split(board: &mut Board, depth: u8) -> u64 {
    let legal_moves = board.legal_moves();
    let mut count: u64 = 0;
    for move_ in legal_moves.iter() {
        board.push(move_);
        let p = perft(board, depth - 1);
        count += p;
        let uci = move_.to_uci_string();
        println!("2. {} moves = {}", uci, p);
        board.pop();
    }
    count
}
