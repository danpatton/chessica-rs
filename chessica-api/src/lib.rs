extern crate chessica;
extern crate chessica_engine;
extern crate libc;

use libc::size_t;
use std::slice;
use chessica::board::Board;
use chessica_engine::search::{Search, TranspositionTable};

unsafe fn to_string(buf: *const u8, len: size_t) -> String {
    assert!(!buf.is_null());
    let slice = slice::from_raw_parts(buf, len as usize);
    String::from_utf8_unchecked(slice.to_vec())
}

#[no_mangle]
pub extern fn get_best_move(
    max_depth: u32,
    tt_key_bits: u32,
    initial_fen_buf: *const u8,
    initial_fen_len: size_t,
    uci_moves_buf: *const u8,
    uci_moves_len: size_t,
    best_move_buf: *mut u8,
    best_move_len: size_t
) -> u32 {
    let initial_fen = unsafe {
        to_string(initial_fen_buf, initial_fen_len)
    };
    let mut board = Board::parse_fen(&initial_fen).unwrap();
    if uci_moves_len > 0 {
        let uci_moves = unsafe {
            to_string(uci_moves_buf, uci_moves_len)
        };
        for uci_move in uci_moves.split(",") {
            if let Err(_) = board.push_uci(uci_move) {
                return 0;
            }
        }
    }
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
