use crate::board::Board;

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
