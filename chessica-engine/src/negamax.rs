use chessica::board::Board;
use chessica::Move;
use crate::negamax::Node::{FailHigh, FailLow, Pv, Terminal};

struct SearchState {
    eval_count: u32,
    cutoff_count: u32,
    killer_cutoff_count: u32,
    non_killer_cutoff_count: u32,
    killer_moves: Vec<Option<Move>>
}

impl SearchState {
    fn new(max_depth: u8) -> Self {
        let mut killer_moves = vec![];
        for _ in 0..max_depth+1 {
            killer_moves.push(None);
        }
        SearchState {
            eval_count: 0,
            cutoff_count: 0,
            killer_cutoff_count: 0,
            non_killer_cutoff_count: 0,
            killer_moves
        }
    }
}

enum Node {
    Terminal(TerminalNode),
    Pv(PvNode),
    FailHigh(FailHighNode),
    FailLow(FailLowNode)
}

impl Node {
    pub fn terminal(score_exact: i32) -> Self {
        Terminal(TerminalNode{ score_exact })
    }
    pub fn pv(pv_moves: Vec<Move>, score_exact: i32) -> Self {
        Pv(PvNode { pv_moves, score_exact })
    }
    pub fn fail_high(score_lower_bound: i32) -> Self {
        FailHigh(FailHighNode { score_lower_bound })
    }
    pub fn fail_low(score_upper_bound: i32) -> Self {
        FailLow(FailLowNode { score_upper_bound })
    }
}

pub struct TerminalNode {
    pub score_exact: i32
}

pub struct PvNode {
    pub pv_moves: Vec<Move>,
    pub score_exact: i32
}

pub struct FailHighNode {
    pub score_lower_bound: i32
}

pub struct FailLowNode {
    pub score_upper_bound: i32
}

fn eval(board: &Board, state: &mut SearchState) -> i32 {
    state.eval_count += 1;
    board.get_negamax_score()
}

fn search(board: &mut Board, depth: u8, state: &mut SearchState) -> Move {
    let alpha = i32::MIN + 1;
    let beta = i32::MAX;
    // negamax_v1(board, depth, state).0.unwrap()
    // negamax_v2(board, depth, state, alpha, beta).0.unwrap()
    let node = negamax_v3(board, depth, state, alpha, beta);
    match node {
        Pv(pv_node) => {
            let moves_str = pv_node.pv_moves
                .iter()
                .map(|m| m.to_uci_string().to_string())
                .collect::<Vec<String>>().join(", ");
            println!("PV moves: {}", moves_str);
            Some(pv_node.pv_moves.first().unwrap().clone())
        },
        _ => None
    }.unwrap()
}

fn negamax_v1(board: &mut Board, depth: u8, state: &mut SearchState) -> (Option<Move>, i32) {
    let moves = board.legal_moves();
    if moves.is_empty() {
        return (None, board.get_negamax_score());
    }
    let mut best_score = i32::MIN + 1;
    let mut best_move: Option<Move> = None;
    for move_ in moves.iter() {
        board.push(move_);
        let score = match depth {
            1 => -eval(board, state),
            _ => -negamax_v1(board, depth - 1, state).1
        };
        board.pop();
        if score > best_score {
            best_score = score;
            best_move = Some(*move_);
        }
    }
    (best_move, best_score)
}

fn negamax_v2(board: &mut Board, depth: u8, state: &mut SearchState, alpha: i32, beta: i32) -> (Option<Move>, i32) {
    let moves = board.legal_moves();
    if moves.is_empty() {
        return (None, board.get_negamax_score());
    }
    let mut local_alpha = alpha;
    let mut best_move: Option<Move> = None;
    for move_ in moves.iter() {
        board.push(move_);
        let score = match depth {
            1 => -eval(board, state),
            _ => -negamax_v2(board, depth - 1, state, -beta, -local_alpha).1
        };
        board.pop();
        if score > local_alpha {
            best_move = Some(*move_);
        }
        local_alpha = local_alpha.max(score);
        if local_alpha > beta {
            break;
        }
    }
    (best_move, local_alpha)
}

fn negamax_v3(board: &mut Board, depth: u8, state: &mut SearchState, alpha: i32, beta: i32) -> Node {
    let mut moves = board.legal_moves();
    if moves.is_empty() {
        return Node::terminal(board.get_negamax_score());
    }
    let killer_move = state.killer_moves[depth as usize];
    if let Some(killer_move) = killer_move {
        if let Some(killer_move_pos) = moves.iter().position(|&m| m == killer_move) {
            moves.swap(0, killer_move_pos);
        }
    }
    let mut score_upper_bound = i32::MAX;
    let mut local_alpha = alpha;
    let mut pv_node_to_return: Option<Node> = None;

    for move_ in moves.iter() {
        board.push(move_);
        let neg_node = match depth {
            1 => Node::pv(vec![], eval(board, state)),
            _ => negamax_v3(board, depth - 1, state, -beta, -local_alpha)
        };
        board.pop();
        match neg_node {
            Terminal(t) => {
                let score = -t.score_exact;
                if score >= local_alpha {
                    local_alpha = score;
                    let pv_moves = vec![*move_];
                    pv_node_to_return = Some(Node::pv(pv_moves, score));
                }
            },
            Pv(pv) => {
                let score = -pv.score_exact;
                if score >= local_alpha {
                    local_alpha = score;
                    let mut pv_moves = vec![*move_];
                    pv_moves.extend(pv.pv_moves.iter());
                    pv_node_to_return = Some(Node::pv(pv_moves, score));
                }
            },
            FailHigh(fh) => {
                score_upper_bound = score_upper_bound.min(-fh.score_lower_bound);
            },
            FailLow(fl) => {
                let score_lower_bound = -fl.score_upper_bound;
                // assert!(score_lower_bound > beta);
                return Node::fail_high(score_lower_bound);
            }
        }
        if local_alpha > beta {
            state.cutoff_count += 1;
            if let Some(killer_move) = killer_move {
                if killer_move == *move_ {
                    state.killer_cutoff_count += 1;
                }
                else {
                    state.non_killer_cutoff_count += 1;
                }
            }
            state.killer_moves[depth as usize] = Some(*move_);
            return Node::fail_high(local_alpha);
        }
    }

    match pv_node_to_return {
        Some(pv_node) => pv_node,
        None => Node::fail_low(score_upper_bound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_finds_mate_in_two() {
        const MAX_DEPTH: u8 = 3;
        let mut board = Board::parse_fen("kr5r/p7/8/8/8/1R2Q3/6q1/KR6 w - - 0 1").unwrap();
        let mut state = SearchState::new(MAX_DEPTH);
        let move_ = search(&mut board, MAX_DEPTH, &mut state);
        println!("Eval count: {}", state.eval_count);
        println!("Cutoff count: {}", state.cutoff_count);
        println!("Killer cutoff count: {}", state.killer_cutoff_count);
        println!("Non-killer cutoff count: {}", state.non_killer_cutoff_count);
        assert_eq!(move_.to_uci_string(), "e3a7");
    }

    #[test_case("4rrk1/pppb4/7p/3P2pq/3Q4/P5P1/1PP2nKP/R3RNN1 b - - 0 1", "d7h3" ; "mate in three #1")]
    #[test_case("r5rk/5p1p/5R2/4B3/8/8/7P/7K w - - 0 1", "f6a6" ; "mate in three #2")]
    #[test_case("2r3k1/p4p2/3Rp2p/1p2P1pK/8/1P4P1/P3Q2P/1q6 b - - 0 1", "b1g6" ; "mate in three #3")]
    #[test_case("1k5r/pP3ppp/3p2b1/1BN1n3/1Q2P3/P1B5/KP3P1P/7q w - - 1 0", "c5a6" ; "mate in three #4")]
    #[test_case("3r4/pR2N3/2pkb3/5p2/8/2B5/qP3PPP/4R1K1 w - - 1 0", "c3e5" ; "mate in three #5")]
    #[test_case("R6R/1r3pp1/4p1kp/3pP3/1r2qPP1/7P/1P1Q3K/8 w - - 1 0", "f4f5" ; "mate in three #6")]
    #[test_case("4r1k1/5bpp/2p5/3pr3/8/1B3pPq/PPR2P2/2R2QK1 b - - 0 1", "e5e1" ; "mate in three #7")]
    #[test_case("2r5/2p2k1p/pqp1RB2/2r5/PbQ2N2/1P3PP1/2P3P1/4R2K w - - 1 0", "e6e7" ; "mate in three #8")]
    fn test_finds_mate_in_three(initial_fen: &str, uci_move: &str) {
        const MAX_DEPTH: u8 = 5;
        let mut board = Board::parse_fen(initial_fen).unwrap();
        let mut state = SearchState::new(MAX_DEPTH);
        let move_ = search(&mut board, MAX_DEPTH, &mut state);
        println!("Eval count: {}", state.eval_count);
        println!("Cutoff count: {}", state.cutoff_count);
        println!("Killer cutoff count: {}", state.killer_cutoff_count);
        println!("Non-killer cutoff count: {}", state.non_killer_cutoff_count);
        assert_eq!(move_.to_uci_string(), uci_move);
    }
}