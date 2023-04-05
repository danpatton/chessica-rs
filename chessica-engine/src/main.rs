#[macro_use]
extern crate chessica;

use chessica::bitboard::BitBoard;
use chessica::square::Square;

fn main() {
    let e4 = sq!(e4);
    let d5 = sq!(d5);
    let bb = BitBoard::empty().set_all(&[e4, d5]);
    println!("{}", bb);
}
