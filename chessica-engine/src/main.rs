extern crate chessica;

use std::env;
use std::process::exit;
use chessica::board::Board;
use chessica::perft::perft;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1) {
        Some(command) => {
            match command.as_str() {
                "perft" => {
                    let maxdepth: u8 = args.get(2).and_then(|d| d.parse().ok()).expect("Usage: perft <depth> <fen>");
                    let fen = args.get(3).expect("Usage: perft <depth> <fen>");
                    let mut board = Board::parse_fen(fen).expect("Invalid Fen");
                    // ensure magic bitboards are initialised
                    perft(&mut board, 1);
                    for i in 0..maxdepth {
                        let depth = i + 1;
                        let start = Instant::now();
                        let moves = perft(&mut board, depth);
                        let duration = start.elapsed();
                        println!("perft({:2})= {:12} ( {:.3} sec)", depth, moves, duration.as_secs_f32());
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
