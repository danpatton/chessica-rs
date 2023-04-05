use std::fmt;
use std::fmt::Formatter;
use crate::square::Square;

use tabled::{builder::Builder, Style};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BitBoard {
    value: u64
}

impl BitBoard {
    pub fn new(value: u64) -> Self {
        BitBoard { value }
    }

    pub fn empty() -> Self {
        BitBoard { value: 0 }
    }

    pub fn count(&self) -> u32 {
        self.value.count_ones()
    }

    pub fn set(&self, square: &Square) -> BitBoard {
        BitBoard { value: self.value | square.bit() }
    }

    pub fn set_all(&self, squares: &[Square]) -> BitBoard {
        let bits: u64 = squares.iter().map(|s| s.bit()).fold(0, |a, b| a | b);
        BitBoard { value: self.value | bits }
    }

    pub fn clear(&self, square: &Square) -> BitBoard {
        BitBoard { value: self.value & !square.bit() }
    }

    pub fn clear_all(&self, squares: &[Square]) -> BitBoard {
        let bits: u64 = squares.iter().map(|s| s.bit()).fold(0, |a, b| a | b);
        BitBoard { value: self.value & !bits }
    }

    pub fn is_occupied(&self, square: &Square) -> bool {
        self.value & square.bit() != 0
    }
}

impl Iterator for BitBoard {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        match self.value {
            0 => None,
            _ => {
                let square = Square { ordinal: self.value.trailing_zeros() as u8 };
                self.value &= !square.bit();
                Some(square)
            }
        }
    }
}

impl fmt::Display for BitBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut builder = Builder::default();
        builder.set_columns(["", "a", "b", "c", "d", "e", "f", "g", "h"]);
        let rank_str = vec!["1", "2", "3", "4", "5", "6", "7", "8"];
        for rank in (0..8).rev() {
            let mut row = vec![rank_str[rank]];
            let squares = (0..8).map(|file| Square::from_coords(rank as u8, file)).collect::<Vec<Square>>();
            for square in squares.iter() {
                let x = if self.is_occupied(square) { "X" } else { "" };
                row.push(x);
            }
            builder.add_record(row);
        }
        let mut table = builder.build();
        table.with(Style::rounded());
        table.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::sq;
    use super::*;

    #[test]
    fn test_accessors() {
        let a3 = sq!(a3);
        let e4 = sq!(e4);
        let h8 = sq!(h8);
        let f5 = sq!(f5);
        let bb = BitBoard::empty().set_all(&[e4, h8]);
        assert_eq!(bb.count(), 2);
        assert_eq!(bb.is_occupied(&e4), true);
        assert_eq!(bb.is_occupied(&h8), true);
        assert_eq!(bb.is_occupied(&f5), false);
        let bb2 = bb.set(&f5);
        assert_eq!(bb2.count(), 3);
        assert_eq!(bb2.is_occupied(&f5), true);
        assert_eq!(bb2.is_occupied(&a3), false);
        let bb3 = bb2.set(&a3);
        assert_eq!(bb3.count(), 4);
        assert_eq!(bb3.is_occupied(&a3), true);
        let bb4 = bb3.clear(&e4);
        assert_eq!(bb4.count(), 3);
        assert_eq!(bb4.is_occupied(&e4), false);
    }

    #[test]
    fn test_iter() {
        let a3 = sq!(a3);
        let e4 = sq!(e4);
        let h8 = sq!(h8);
        let f5 = sq!(f5);
        let bb = BitBoard::empty().set_all(&[e4, f5, h8]);
        assert_eq!(&bb.collect::<Vec<Square>>(), &[e4, f5, h8]);
        let bb2 = bb.set(&a3).clear(&f5);
        assert_eq!(&bb2.collect::<Vec<Square>>(), &[a3, e4, h8]);
    }
}