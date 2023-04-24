use chessica::board::Board;
use chessica::Move;
use crate::negamax::Node::{FailHigh, FailLow, Pv, Terminal};

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

struct SearchStats {
    eval_count: u32
}

fn eval(board: &Board, stats: &mut SearchStats) -> i32 {
    stats.eval_count += 1;
    board.get_negamax_score()
}

fn search(board: &mut Board, depth: u8, stats: &mut SearchStats) -> Move {
    let alpha = i32::MIN + 1;
    let beta = i32::MAX;
    // negamax_v1(board, depth, stats).0.unwrap()
    // negamax_v2(board, depth, stats, alpha, beta).0.unwrap()
    let node = negamax_v3(board, depth, stats, alpha, beta);
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

fn negamax_v1(board: &mut Board, depth: u8, stats: &mut SearchStats) -> (Option<Move>, i32) {
    let moves = board.legal_moves();
    if moves.is_empty() {
        return (None, board.get_negamax_score());
    }
    let mut best_score = i32::MIN + 1;
    let mut best_move: Option<Move> = None;
    for move_ in moves.iter() {
        board.push(move_);
        let score = match depth {
            1 => -eval(board, stats),
            _ => -negamax_v1(board, depth - 1, stats).1
        };
        board.pop();
        if score > best_score {
            best_score = score;
            best_move = Some(*move_);
        }
    }
    (best_move, best_score)
}

fn negamax_v2(board: &mut Board, depth: u8, stats: &mut SearchStats, alpha: i32, beta: i32) -> (Option<Move>, i32) {
    let moves = board.legal_moves();
    if moves.is_empty() {
        return (None, board.get_negamax_score());
    }
    let mut local_alpha = alpha;
    let mut best_move: Option<Move> = None;
    for move_ in moves.iter() {
        board.push(move_);
        let score = match depth {
            1 => -eval(board, stats),
            _ => -negamax_v2(board, depth - 1, stats, -beta, -local_alpha).1
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

fn negamax_v3(board: &mut Board, depth: u8, stats: &mut SearchStats, alpha: i32, beta: i32) -> Node {
    let moves = board.legal_moves();
    if moves.is_empty() {
        return Node::terminal(board.get_negamax_score());
    }
    let mut score_upper_bound = i32::MAX;
    let mut local_alpha = alpha;
    let mut pv_node_to_return: Option<Node> = None;

    for move_ in moves.iter() {
        board.push(move_);
        let neg_node = match depth {
            1 => Node::pv(vec![], eval(board, stats)),
            _ => negamax_v3(board, depth - 1, stats, -beta, -local_alpha)
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

    #[test]
    fn test_finds_mate_in_two() {
        let mut board = Board::parse_fen("kr5r/p7/8/8/8/1R2Q3/6q1/KR6 w - - 0 1").unwrap();
        let mut stats = SearchStats {
            eval_count: 0
        };
        let move_ = search(&mut board, 3, &mut stats);
        println!("Eval count: {}", stats.eval_count);
        assert_eq!(move_.to_uci_string(), "e3a7");
    }

    #[test]
    fn test_finds_mate_in_three() {
        let mut board = Board::parse_fen("4rrk1/pppb4/7p/3P2pq/3Q4/P5P1/1PP2nKP/R3RNN1 b - - 0 1").unwrap();
        let mut stats = SearchStats {
            eval_count: 0
        };
        let move_ = search(&mut board, 5, &mut stats);
        println!("Eval count: {}", stats.eval_count);
        assert_eq!(move_.to_uci_string(), "d7h3");
    }
}