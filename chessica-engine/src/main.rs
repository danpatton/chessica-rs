extern crate chessica;

use chessica::board::Board;
use chessica::magic::{find_fancy_bishop_magics, find_fancy_rook_magics};
use chessica::perft::{perft, perft_h, PerftHashEntry};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, stdin, stdout};
use std::process::exit;
use std::time::Instant;
use log::LevelFilter;
use simplelog::{CombinedLogger, Config, WriteLogger};
use chessica::Side;
use chessica_engine::search::{Search, TranspositionTable};
use chessica_engine::uci::UciSession;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1) {
        Some(command) => {
            match command.as_str() {
                "selfplay" => {
                    let mut board = Board::starting_position();
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
                        let mut tt = TranspositionTable::new(24);
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
                },
                "bmagics" => {
                    let start = Instant::now();
                    let bishop_magics = find_fancy_bishop_magics(5, 1_000_000);
                    let duration = start.elapsed();
                    println!(
                        "Found {} bishop magics ({:.3} sec)",
                        bishop_magics.len(),
                        duration.as_secs_f32()
                    );
                }
                "rmagics" => {
                    let start = Instant::now();
                    let rook_magics = find_fancy_rook_magics(10, 1_000_000);
                    let duration = start.elapsed();
                    println!(
                        "Found {} rook magics ({:.3} sec)",
                        rook_magics.len(),
                        duration.as_secs_f32()
                    );
                }
                "perft" => {
                    if args.len() < 4 || args.len() > 5 {
                        println!("Usage: perft <max_depth> [-H<hash_bits>] <fen>");
                        exit(-1);
                    }
                    let max_depth: u8 = args.get(2).and_then(|d| d.parse().ok()).unwrap();
                    if max_depth > 7 {
                        println!("Error: max_depth cannot exceed 7");
                        exit(-1);
                    }
                    let arg3 = args.get(3).unwrap();
                    if arg3.starts_with("-H") {
                        let hash_bits: u8 = arg3[2..].parse().unwrap();
                        if hash_bits > 30 {
                            println!("Error: hash_bits cannot exceed 30");
                            exit(-1);
                        }
                        let fen = args.get(4).unwrap();
                        let mut board = Board::parse_fen(fen).expect("Invalid fen");
                        // ensure magic bitboards are initialised
                        perft(&mut board, 1);
                        let mut hash_table = vec![PerftHashEntry(0, 0); 1 << hash_bits];
                        for i in 0..max_depth {
                            let depth = i + 1;
                            let start = Instant::now();
                            let moves = perft_h(&mut board, depth, &mut hash_table);
                            let duration = start.elapsed();
                            println!(
                                "perft({:2})= {:12} ( {:.3} sec)",
                                depth,
                                moves,
                                duration.as_secs_f32()
                            );
                        }
                    } else {
                        let fen = arg3;
                        let mut board = Board::parse_fen(fen).expect("Invalid fen");
                        // ensure magic bitboards are initialised
                        perft(&mut board, 1);
                        for i in 0..max_depth {
                            let depth = i + 1;
                            let start = Instant::now();
                            let moves = perft(&mut board, depth);
                            let duration = start.elapsed();
                            println!(
                                "perft({:2})= {:12} ( {:.3} sec)",
                                depth,
                                moves,
                                duration.as_secs_f32()
                            );
                        }
                    }
                }
                _ => {
                    println!("Unknown command: {}", command);
                    exit(-1);
                }
            }
        }
        _ => {
            CombinedLogger::init(
                vec![
                    WriteLogger::new(LevelFilter::Info, Config::default(), File::create("/home/dan/logs/chessica_engine.log").unwrap()),
                ]
            ).unwrap();
            let output = Box::new(BufWriter::new(stdout()));
            let mut uci_session = UciSession::new(output);
            let mut input: Box<dyn BufRead> = Box::new(BufReader::new(stdin()));
            uci_session.run(&mut input);
        }
    }
}
