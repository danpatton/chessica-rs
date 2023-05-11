extern crate chessica;
extern crate chessica_engine;
extern crate libc;

use libc::size_t;
use std::slice;
use chessica::board::Board;
use chessica_engine::search::{Search, TranspositionTable};

#[no_mangle]
pub extern fn get_best_move(max_depth: u32, tt_key_bits: u32, fen_buf: *const u8, fen_len: size_t, best_move_buf: *mut u8, best_move_len: size_t) -> u32 {
    let fen = unsafe {
        assert!(!fen_buf.is_null());
        let fen_slice = slice::from_raw_parts(fen_buf, fen_len as usize);
        String::from_utf8_unchecked(fen_slice.to_vec())
    };
    let board = Board::parse_fen(&fen).unwrap();
    let mut tt = TranspositionTable::new(tt_key_bits as u8);
    let mut search = Search::new(max_depth as usize);
    match  search.search(&board, &mut tt) {
        Some(best_move) => {
            let best_move_uci = best_move.to_uci_string();
            let best_move_slice = unsafe {
                assert!(!best_move_buf.is_null());
                slice::from_raw_parts_mut(best_move_buf, best_move_len as usize)
            };
            let best_move_chars = best_move_uci.as_bytes();
            for i in 0..best_move_chars.len() {
                best_move_slice[i] = best_move_chars[i];
            }
            best_move_chars.len() as u32
        },
        None => 0
    }
}
