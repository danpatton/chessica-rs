extern crate chessica;
extern crate chessica_engine;
extern crate libc;

use libc::c_char;
use std::ffi::{CStr, CString};
use std::ptr::null_mut;
use chessica::board::Board;
use chessica_engine::search::{Search, TranspositionTable};

#[no_mangle]
pub extern fn get_best_move(
    initial_fen: *const c_char,
    uci_moves: *const c_char,
    max_depth: u8,
    tt_key_bits: u8,
    rng_seed: u64
) -> *mut c_char {
    if initial_fen.is_null() || uci_moves.is_null() {
        return null_mut();
    }
    let initial_fen = unsafe {
        CStr::from_ptr(initial_fen)
    };
    let initial_fen_str = initial_fen.to_str().unwrap();
    let uci_moves = unsafe {
        CStr::from_ptr(uci_moves)
    };
    let uci_moves_str = uci_moves.to_str().unwrap();
    let mut board = Board::parse_fen(initial_fen_str).unwrap();
    if uci_moves_str.len() > 0 {
        for uci_move in uci_moves_str.split(",") {
            if let Err(_) = board.push_uci(uci_move) {
                return null_mut();
            }
        }
    }
    let mut tt = TranspositionTable::new(tt_key_bits);
    let mut search = Search::new_with_rng(max_depth as usize, rng_seed);
    match search.search(&board, &mut tt) {
        Some(best_move) => {
            let result = best_move.to_uci_string();
            let c_str_result  = CString::new(result).unwrap();
            c_str_result.into_raw()
        },
        None => null_mut()
    }
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        CString::from_raw(s)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_best_move() {
        let initial_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
        let uci_moves = "".to_string();
        let initial_fen_cstr = CString::new(initial_fen).unwrap();
        let uci_moves_cstr = CString::new(uci_moves).unwrap();
        let best_move_ptr = get_best_move(initial_fen_cstr.into_raw(), uci_moves_cstr.into_raw(),  5, 20, 0);
        let best_move_str = unsafe {
            CStr::from_ptr(best_move_ptr)
        }.to_str().unwrap();
        assert_eq!(best_move_str, "g1f3");
    }
}