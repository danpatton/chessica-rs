use std::ops;
use itertools::Itertools;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use chessica::board::Board;
use chessica::Move;
use crate::search::Score::{LowerBound, UpperBound, Exact};

#[derive(Debug, Copy, Clone)]
pub enum Score {
    LowerBound(i16),
    UpperBound(i16),
    Exact(i16)
}

impl ops::Neg for Score {
    type Output = Score;

    fn neg(self) -> Self::Output {
        match self {
            Exact(s) => Exact(-s),
            LowerBound(s) => UpperBound(-s),
            UpperBound(s) => LowerBound(-s)
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum TTScore {
    Alpha(i16),
    Beta(i16, Move),
    Pv(i16, Move),
    None
}

#[derive(Debug, Copy, Clone)]
struct TTEntry {
    position_hash: u64,
    depth: u8,
    score: TTScore,
}

impl TTEntry {
    fn empty() -> Self {
        TTEntry {
            position_hash: 0,
            depth: 0,
            score: TTScore::None
        }
    }
}

pub struct TranspositionTable {
    size: usize,
    entries: Vec<TTEntry>
}

impl TranspositionTable {
    pub fn new(key_bits: u8) -> TranspositionTable {
        let size: usize = 43 + (1 << key_bits);
        TranspositionTable {
            size,
            entries: vec![TTEntry::empty(); size as usize]
        }
    }

    fn put(&mut self, position: &Board, depth: u8, score: TTScore) {
        let idx = position.hash() as usize % self.size;
        let entry = &mut self.entries[idx];
        if entry.position_hash == position.hash() && depth < entry.depth {
            return;
        }
        entry.position_hash = position.hash();
        entry.depth = depth;
        entry.score = score;
    }

    fn get(&self, position: &Board, depth: u8, alpha: i16, beta: i16) -> Option<Score> {
        let idx = position.hash() as usize % self.size;
        let entry = &self.entries[idx];
        if entry.depth >= depth && entry.position_hash == position.hash() {
            return match entry.score {
                TTScore::Pv(score, _) => Some(Exact(score)),
                TTScore::Alpha(score) if score <= alpha => Some(UpperBound(score)),
                TTScore::Beta(score, _) if score >= beta => Some(LowerBound(score)),
                _ => None
            }
        }
        None
    }

    fn get_pv_move(&self, position: &Board) -> Option<Move> {
        let idx = position.hash() as usize % self.size;
        let entry = &self.entries[idx];
        if entry.position_hash == position.hash() {
            return match entry.score {
                TTScore::Pv(_, move_) => Some(move_),
                _ => None
            }
        }
        None
    }

    fn get_move(&self, position: &Board) -> Option<Move> {
        let idx = position.hash() as usize % self.size;
        let entry = &self.entries[idx];
        if entry.position_hash == position.hash() {
            return match entry.score {
                TTScore::Pv(_, move_) => Some(move_),
                TTScore::Beta(_, move_) => Some(move_),
                _ => None
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.entries.fill(TTEntry::empty());
    }
}

pub struct Search {
    max_depth: usize,
    eval_count: u32,
    cutoff_count: u32,
    q_cutoff_count: u32,
    tt_hit_count: u32,
    last_pv: Vec<Move>,
    pv_table: Vec<Vec<Move>>,
    rng_seed: u64,
    rng: StdRng
}

impl Search {
    pub fn new(max_depth: usize) -> Self {
        Search::new_with_rng(max_depth, 0)
    }

    pub fn new_with_rng(max_depth: usize, rng_seed: u64) -> Self {
        let mut pv_table = vec![];
        for i in 0..max_depth {
            pv_table.push(Vec::with_capacity(max_depth - i));
        }
        Search {
            max_depth,
            eval_count: 0,
            cutoff_count: 0,
            q_cutoff_count: 0,
            tt_hit_count: 0,
            last_pv: vec![],
            pv_table,
            rng_seed,
            rng: StdRng::seed_from_u64(rng_seed)
        }
    }

    fn _eval(&mut self, board: &Board) -> i16 {
        self.eval_count += 1;

        // TODO: work out how to use traits (?) to make eval function pluggable

        if board.is_in_check() && board.legal_moves().is_empty() {
            // checkmate!
            return -30_000;
        }
        else if board.is_draw_by_threefold_repetition() {
            return 0;
        }
        else if board.is_draw_by_fifty_move_rule() {
            return 0;
        }

        let perturbation = match self.rng_seed {
            // seed of 0 means "don't perturb"
            0 => 0,
            _ => self.rng.gen_range(-5..5)
        };

        board.get_pst_negamax_score() + perturbation
    }

    fn _qsearch(&mut self, board: &mut Board, alpha: i16, beta: i16) -> Score {
        let in_check = board.is_in_check();

        let mut alpha = alpha;
        let mut is_pv = false;

        let stand_pat_score = self._eval(board);
        if !board.is_in_check() {
            if stand_pat_score > alpha {
                is_pv = true;
                alpha = stand_pat_score;
                if alpha >= beta {
                    self.q_cutoff_count += 1;
                    return LowerBound(alpha);
                }
            }
        }

        let mut moves = board.legal_moves();

        if moves.is_empty() {
            if !in_check {
                // stalemate!
                return Exact(0)
            }
            return Exact(stand_pat_score);
        }

        if !in_check {
            // move ordering: SEE
            let sort_keys = moves.iter().map(|&m| if m.is_capture() {
                board.static_exchange_score(m)
            } else if board.is_passed_pawn(m.from()) {
                1
            } else {
                0
            });
            moves = moves.iter()
                .zip(sort_keys)
                .filter(|&t| t.1 > 0)
                .sorted_by_key(|&t| -t.1).map(|(&m, _)| m)
                .collect();
        }

        for &move_ in moves.iter() {
            board.push(&move_);
            let score = -self._qsearch(board, -beta, -alpha);
            board.pop();
            match score {
                LowerBound(score) => {
                    // the node we've just searched was an all-node => this move is too good; we
                    // will never get the chance to play it since our opponent will never make the
                    // move that led to this position
                    self.q_cutoff_count += 1;
                    return LowerBound(score);
                },
                UpperBound(_) => {
                    // the node we've just searched was a cut-node => this move is bad; we will
                    // never play it
                },
                Exact(score) => {
                    // the node we've just searched was a pv-node
                    if score > alpha {
                        is_pv = true;
                        alpha = score;
                    }
                }
            }
            if alpha >= beta {
                self.q_cutoff_count += 1;
                return LowerBound(alpha);
            }
        }

        if is_pv {
            Exact(alpha)
        } else {
            UpperBound(alpha)
        }
    }

    fn _search(&mut self, board: &mut Board, tt: &mut TranspositionTable, depth: usize, pv_idx: usize, alpha: i16, beta: i16) -> Score {

        if depth == 0 {
            return self._qsearch(board, alpha, beta);
        }

        if let Some(tt_score) = tt.get(board, depth as u8, alpha, beta) {
            self.tt_hit_count += 1;
            self.pv_table[pv_idx].truncate(0);
            if let Some(pv_move) = tt.get_pv_move(board) {
                self.pv_table[pv_idx].push(pv_move);
            }
            return tt_score;
        }

        let mut moves = board.legal_moves();
        if moves.is_empty() {
            self.pv_table[pv_idx].truncate(0);
            let score = if board.is_in_check() { -30_000 } else { 0 };
            return Exact(score);
        }

        // move ordering: hash move first, then PV move from previous search, then captures in MVV/LVA order
        // TODO: killer move heuristic?
        let mut sort_from = 0;
        if let Some(hash_move) = tt.get_move(board) {
            if let Some(h) = moves.iter().skip(sort_from).position(|&m| m == hash_move) {
                if h != sort_from {
                    moves.swap(sort_from, h);
                    sort_from += 1;
                }
            }
        }
        if pv_idx < self.last_pv.len() {
            let prev_pv_move = self.last_pv[pv_idx];
            if let Some(pv) = moves.iter().skip(sort_from).position(|&m| m == prev_pv_move) {
                if pv != sort_from {
                    moves.swap(sort_from, pv);
                    sort_from += 1;
                }
            }
        }
        // MVV-LVA
        moves.as_mut_slice()[sort_from..].sort_by_key(|m| (-m.capture_value(), m.piece().value()));

        let mut alpha = alpha;
        let mut pv_move: Option<Move> = None;

        for &move_ in moves.iter() {
            board.push(&move_);
            let score = -self._search(board, tt, depth - 1, pv_idx + 1, -beta, -alpha);
            board.pop();
            match score {
                LowerBound(score) => {
                    // the node we've just searched was an all-node => this move is too good; we
                    // will never get the chance to play it since our opponent will never make the
                    // move that led to this position
                    self.cutoff_count += 1;
                    tt.put(board, depth as u8, TTScore::Beta(score, move_));
                    return LowerBound(score);
                },
                UpperBound(_) => {
                    // the node we've just searched was a cut-node => this move is bad; we will
                    // never play it
                },
                Exact(score) => {
                    // the node we've just searched was a pv-node
                    if score > alpha {
                        alpha = score;
                        if score < beta {
                            pv_move = Some(move_);
                        }
                    }
                }
            }
            if alpha >= beta {
                self.cutoff_count += 1;
                let score = LowerBound(beta);
                tt.put(board, depth as u8, TTScore::Beta(beta, move_));
                return score;
            }
        }

        let score = if let Some(pv_move) = pv_move {
            let (head, tail) = self.pv_table.split_at_mut(pv_idx + 1);
            let pv = &mut head[pv_idx];
            pv.truncate(0);
            pv.push(pv_move);
            if !tail.is_empty() {
                let pv_tail = &tail[0];
                pv.extend(pv_tail.iter());
            }
            tt.put(board, depth as u8, TTScore::Pv(alpha, pv_move));
            Exact(alpha)
        }
        else {
            tt.put(board, depth as u8, TTScore::Alpha(alpha));
            UpperBound(alpha)
        };

        score
    }

    pub fn search(&mut self, board: &Board, tt: &mut TranspositionTable) -> Option<Move> {
        let mut board = board.clone();
        for i in 0..self.max_depth {
            let search_depth = i + 1;
            let alpha = -i16::MAX;
            let beta = i16::MAX;
            self._search(&mut board, tt, search_depth, 0, alpha, beta);
            self.last_pv = self.pv_table[0].clone();
            for i in 0..self.max_depth {
                self.pv_table[i].truncate(0);
            }
        }
        self.last_pv.first().map(|&m| m)
    }

    pub fn get_pv(&self) -> Vec<Move> {
        self.last_pv.clone()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("kr5r/p7/8/8/8/1R2Q3/6q1/KR6 w - - 0 1", 3, "e3a7" ; "mate in two #1")]
    #[test_case("4rrk1/pppb4/7p/3P2pq/3Q4/P5P1/1PP2nKP/R3RNN1 b - - 0 1", 5, "d7h3" ; "mate in three #1")]
    #[test_case("r5rk/5p1p/5R2/4B3/8/8/7P/7K w - - 0 1", 5, "f6a6" ; "mate in three #2")]
    #[test_case("2r3k1/p4p2/3Rp2p/1p2P1pK/8/1P4P1/P3Q2P/1q6 b - - 0 1", 5, "b1g6" ; "mate in three #3")]
    #[test_case("1k5r/pP3ppp/3p2b1/1BN1n3/1Q2P3/P1B5/KP3P1P/7q w - - 1 0", 5, "c5a6" ; "mate in three #4")]
    #[test_case("3r4/pR2N3/2pkb3/5p2/8/2B5/qP3PPP/4R1K1 w - - 1 0", 5, "c3e5" ; "mate in three #5")]
    #[test_case("R6R/1r3pp1/4p1kp/3pP3/1r2qPP1/7P/1P1Q3K/8 w - - 1 0", 5, "f4f5" ; "mate in three #6")]
    #[test_case("4r1k1/5bpp/2p5/3pr3/8/1B3pPq/PPR2P2/2R2QK1 b - - 0 1", 5, "e5e1" ; "mate in three #7")]
    #[test_case("2r5/2p2k1p/pqp1RB2/2r5/PbQ2N2/1P3PP1/2P3P1/4R2K w - - 1 0", 5, "e6e7" ; "mate in three #8")]
    // #[test_case("3rr1k1/pp3ppp/3b4/2p5/2Q5/6qP/PPP1B1P1/R1B2K1R b - - 0 1", 7, "g3e1" ; "mate in four #1")]
    fn test_finds_mate(initial_fen: &str, max_depth: usize, uci_move: &str) {
        const TT_BITS: u8 = 24;
        let board = Board::parse_fen(initial_fen).unwrap();
        let mut tt = TranspositionTable::new(TT_BITS);
        let mut search = Search::new_with_rng(max_depth, 342044983);
        let best_move = search.search(&board, &mut tt);
        let pv = search.get_pv();
        let pv_str = pv.iter().map(|&m| m.to_uci_string()).collect::<Vec<String>>().join(", ");
        println!("PV: {}", pv_str);
        println!("Eval count: {}", search.eval_count);
        println!("Cutoff count: {}", search.cutoff_count);
        println!("Q-Cutoff count: {}", search.q_cutoff_count);
        println!("TT hit count: {}", search.tt_hit_count);
        match best_move {
            Some(move_) => assert_eq!(move_.to_uci_string(), uci_move),
            None => assert!(false)
        }
    }

    #[test]
    fn test_treats_passed_pawns_with_respect() {
        let mut board = Board::parse_fen("5N2/1k6/7p/8/1p6/2p4P/6PK/R7 w - - 0 37").unwrap();
        // black has two very dangerous passed pawns; qsearch should ensure we don't ignore the threat
        let mut search = Search::new(7);
        let mut tt = TranspositionTable::new(20);
        match search.search(&board, &mut tt) {
            Some(best_move) => {
                let pv = search.get_pv();
                let pv_str = pv.iter().map(|&m| m.to_uci_string()).collect::<Vec<String>>().join(", ");
                println!("PV: {}", pv_str);
                assert_eq!(best_move.to_uci_string(), "a1b1");
                board.push(&best_move);
            }
            None => assert!(false)
        };
    }

    #[test]
    fn test_avoids_draw_by_threefold_repetition_when_ahead() {
        // white is up a knight
        let mut board = Board::parse_fen("5k2/3R4/8/3N1p1p/8/6KP/3r1P2/8 w - - 2 62").unwrap();

        // white moves king from g3 to f3
        {
            let mut search = Search::new(5);
            let mut tt = TranspositionTable::new(20);
            match search.search(&board, &mut tt) {
                Some(best_move) => {
                    assert_eq!(best_move.to_uci_string(), "g3f3");
                    board.push(&best_move);
                }
                None => assert!(false)
            };
        }

        // black moves rook from d2 to d4
        board.push_uci("d2d4").unwrap();

        // white moves king back from f3 to g3
        {
            let mut search = Search::new(5);
            let mut tt = TranspositionTable::new(20);
            match search.search(&board, &mut tt) {
                Some(best_move) => {
                    assert_eq!(best_move.to_uci_string(), "f3g3");
                    board.push(&best_move);
                }
                None => assert!(false)
            };
        }

        // black moves rook back from d4 to d2
        board.push_uci("d4d2").unwrap();

        // *** we are now in the same position we started in ***

        // white moves king from g3 to f3 again
        {
            let mut search = Search::new(5);
            let mut tt = TranspositionTable::new(20);
            match search.search(&board, &mut tt) {
                Some(best_move) => {
                    assert_eq!(best_move.to_uci_string(), "g3f3");
                    board.push(&best_move);
                }
                None => assert!(false)
            };
        }

        // black moves rook from d2 to d4 again (playing for a draw by repetition, since they are down a knight)
        board.push_uci("d2d4").unwrap();

        // if white now moves king from f3 to g3 again, black will simply move their rook from d4 back to d2, and claim draw by repetition
        // so instead, white moves king from f3 to e2
        {
            let mut search = Search::new(5);
            let mut tt = TranspositionTable::new(20);
            match search.search(&board, &mut tt) {
                Some(best_move) => {
                    assert_eq!(best_move.to_uci_string(), "f3e2");
                    board.push(&best_move);
                }
                None => assert!(false)
            };
        }
    }
}