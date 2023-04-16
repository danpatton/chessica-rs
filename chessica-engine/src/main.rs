extern crate chessica;

use std::env;
use std::process::exit;
use chessica::board::Board;
use chessica::perft::{perft, perft_h, PerftHashEntry};
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1) {
        Some(command) => {
            match command.as_str() {
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
                            println!("perft({:2})= {:12} ( {:.3} sec)", depth, moves, duration.as_secs_f32());
                        }
                    }
                    else {
                        let fen = arg3;
                        let mut board = Board::parse_fen(fen).expect("Invalid fen");
                        // ensure magic bitboards are initialised
                        perft(&mut board, 1);
                        for i in 0..max_depth {
                            let depth = i + 1;
                            let start = Instant::now();
                            let moves = perft(&mut board, depth);
                            let duration = start.elapsed();
                            println!("perft({:2})= {:12} ( {:.3} sec)", depth, moves, duration.as_secs_f32());
                        }
                    }
                },
                _ => {
                    println!("Unknown command: {}", command);
                    exit(-1);
                }
            }
        },
        _ => {
            println!("Usage: <command> [args]");
            exit(-1);
        }
    }
}
