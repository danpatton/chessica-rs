extern crate chessica;

use chessica::board::Board;
use chessica::perft::perft;
use std::time::Instant;

fn main() {
    let mut board = Board::starting_position();

    // ensure magic bitboards are initialised
    print!("Initialising... ");
    perft(&mut board, 2);
    println!("complete");

    let start = Instant::now();
    let answer = perft(&mut board, 7);
    let duration = start.elapsed();

    let duration_s = duration.as_secs_f32();
    let nps = (answer as f64) / (duration.as_micros() as f64);

    println!(
        "Computed perft {} in {:.2}s ({:.1} MNPS)",
        answer, duration_s, nps
    );
}
