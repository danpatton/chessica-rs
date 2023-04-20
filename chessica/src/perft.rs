use crate::board::Board;
use crate::Move;

#[derive(Clone, Copy)]
pub struct PerftHashEntry(pub u64, pub u64);

struct VecChain {
    current: Vec<Move>,
    next: Option<Box<VecChain>>
}

impl VecChain {
    fn build(length: u8, capacity: usize) -> Self {
        let tail = VecChain {
            current: Vec::with_capacity(capacity),
            next: None
        };
        let mut head = tail;
        for _ in 1..length {
            head = VecChain {
                current: Vec::with_capacity(capacity),
                next: Some(Box::new(head))
            };
        }
        head
    }
}

pub fn perft(board: &mut Board, depth: u8) -> u64 {
    fn _impl(board: &mut Board, chain: &mut VecChain) -> u64 {
        chain.current.truncate(0);
        let n_moves = board.legal_moves_noalloc(&mut chain.current);
        match &mut chain.next {
            None => n_moves as u64,
            Some(next_chain) => {
                let mut count = 0u64;
                for move_ in chain.current.iter() {
                    board.push(move_);
                    count += _impl(board, next_chain);
                    board.pop();
                }
                count
            },
        }
    }
    let mut chain = VecChain::build(depth, 100);
    _impl(board, &mut chain)
}

pub fn perft_h(board: &mut Board, depth: u8, hash_table: &mut Vec<PerftHashEntry>) -> u64 {
    assert!(depth <= 7);
    assert!(hash_table.len().is_power_of_two());
    assert!(hash_table.len() >= 16);
    let depth_shift = (hash_table.len().trailing_zeros() - 3) as u8;
    let hash_mask = (1u64 << depth_shift) - 1;

    fn _impl(board: &mut Board, chain: &mut VecChain, depth: u64, depth_shift: u8, hash_mask: u64, hash_table: &mut Vec<PerftHashEntry>) -> u64 {
        let hash = board.hash();
        let idx = ((depth << depth_shift) | (hash & hash_mask)) as usize;
        let entry = &hash_table[idx];
        if entry.0 == hash {
            return entry.1;
        }
        chain.current.truncate(0);
        let n_moves = board.legal_moves_noalloc(&mut chain.current);
        let result = match &mut chain.next {
            None => n_moves as u64,
            Some(next_chain) => {
                let mut count = 0u64;
                for move_ in chain.current.iter() {
                    board.push(move_);
                    count += _impl(board, next_chain, depth - 1, depth_shift, hash_mask, hash_table);
                    board.pop();
                }
                count
            }
        };
        let entry = &mut hash_table[idx];
        *entry = PerftHashEntry(hash, result);
        result
    }

    let mut chain = VecChain::build(depth, 100);
    _impl(board, &mut chain, depth as u64, depth_shift, hash_mask, hash_table)
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
