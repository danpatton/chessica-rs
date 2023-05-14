use chessica::board::Board;
use chessica::Side;
use chessica_engine::search::{Search, TranspositionTable};

pub fn selfplay() {
    let mut board = Board::starting_position();
    let mut tt = TranspositionTable::new(24);
    loop {
        if board.is_draw_by_threefold_repetition() {
            if board.side_to_move() == Side::Black {
                println!();
            }
            println!("1/2-1/2 {{draw by threefold repetition}}");
            break;
        }
        if board.is_draw_by_fifty_move_rule() {
            if board.side_to_move() == Side::Black {
                println!();
            }
            println!("1/2-1/2 {{draw by fifty move rule}}");
            break;
        }
        let mut search = Search::new(5);
        match search.search(&board, &mut tt) {
            Some(move_) => {
                if board.side_to_move() == Side::White {
                    print!("{}. {}", board.full_move_number(), move_.pgn_spec(&board));
                }
                else {
                    println!(" {}", move_.pgn_spec(&board));
                }
                board.push(&move_);
            },
            None => {
                if board.side_to_move() == Side::Black {
                    println!();
                }
                if board.is_in_check() {
                    let result_spec = match board.side_to_move() {
                        Side::White => "0-1",
                        Side::Black => "1-0"
                    };
                    println!("{}", result_spec);
                }
                else {
                    println!("1/2-1/2 {{draw by stalemate}}");
                }
                break;
            }
        }
    }
}