use std::fmt;
use std::fmt::Formatter;
use std::ops;
use crate::square::Square;
use crate::bitboard_masks::{FILE, RANK};

use tabled::{builder::Builder, Style};
use crate::Side;
use crate::Side::White;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BitBoard {
    pub value: u64
}

impl BitBoard {
    pub fn new(value: u64) -> Self {
        BitBoard { value }
    }

    pub fn empty() -> Self {
        BitBoard { value: 0 }
    }

    pub fn full() -> Self {
        BitBoard { value: u64::MAX }
    }

    pub fn rank(i: u8) -> Self {
        BitBoard { value: RANK[i as usize] }
    }

    pub fn file(i: u8) -> Self {
        BitBoard { value: FILE[i as usize] }
    }

    pub fn count(self) -> u32 {
        self.value.count_ones()
    }

    pub fn set(self, square: Square) -> BitBoard {
        BitBoard { value: self.value | square.bit() }
    }

    pub fn set_all(self, squares: &[Square]) -> BitBoard {
        let bits: u64 = squares.iter().map(|s| s.bit()).fold(0, |a, b| a | b);
        BitBoard { value: self.value | bits }
    }

    pub fn clear(self, square: Square) -> BitBoard {
        BitBoard { value: self.value & !square.bit() }
    }

    pub fn clear_all(self, squares: &[Square]) -> BitBoard {
        let bits: u64 = squares.iter().map(|s| s.bit()).fold(0, |a, b| a | b);
        BitBoard { value: self.value & !bits }
    }

    pub fn is_occupied(self, square: Square) -> bool {
        self.value & square.bit() != 0
    }

    pub fn is_empty(self) -> bool {
        self.value == 0
    }

    pub fn subsets(self) -> BitBoardSubSetsIterator {
        BitBoardSubSetsIterator::new(self)
    }

    pub fn any(self) -> bool {
        self.value != 0
    }

    pub fn single(self) -> Square {
        match self.value.count_ones() {
            1 => Square::from_ordinal(self.value.trailing_zeros() as u8),
            0 => panic!("tried to call .single on an empty bitboard"),
            _ => panic!("tried to call .single on a bitboard with multiple bits set")
        }
    }

    pub fn magic_hash_index(self, magic: u64, index_shift: u8) -> usize {
        (self.value.wrapping_mul(magic) >> index_shift) as usize
    }

    pub fn pawn_pushes(self, side: Side) -> BitBoard {
        match side {
            White => BitBoard { value: self.value << 8 },
            _ => BitBoard { value: self.value >> 8 }
        }
    }

    pub fn pawn_left_captures(self, side: Side) -> BitBoard {
        match side {
            White => BitBoard { value: (self.value & !FILE[0]) << 7 },
            _ => BitBoard { value: (self.value & !FILE[7]) >> 7 }
        }
    }

    pub fn pawn_right_captures(self, side: Side) -> BitBoard {
        match side {
            White => BitBoard { value: (self.value & !FILE[7]) << 9 },
            _ => BitBoard { value: (self.value & !FILE[0]) >> 9 }
        }
    }

    pub fn pawn_captures(self, side: Side) -> BitBoard {
        self.pawn_left_captures(side) | self.pawn_right_captures(side)
    }
}

impl ops::BitAnd for BitBoard {
    type Output = BitBoard;

    fn bitand(self, rhs: Self) -> Self::Output {
        BitBoard { value: self.value & rhs.value }
    }
}

impl ops::BitAnd<Square> for BitBoard {
    type Output = BitBoard;

    fn bitand(self, rhs: Square) -> Self::Output {
        BitBoard { value: self.value & rhs.bit() }
    }
}

impl ops::BitAndAssign for BitBoard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.value &= rhs.value;
    }
}

impl ops::BitAndAssign<Square> for BitBoard {
    fn bitand_assign(&mut self, rhs: Square) {
        self.value &= rhs.bit();
    }
}

impl ops::Not for BitBoard {
    type Output = BitBoard;

    fn not(self) -> Self::Output {
        BitBoard { value: !self.value }
    }
}

impl ops::BitOr for BitBoard {
    type Output = BitBoard;

    fn bitor(self, rhs: Self) -> Self::Output {
        BitBoard { value: self.value | rhs.value }
    }
}

impl ops::BitOr<Square> for BitBoard {
    type Output = BitBoard;

    fn bitor(self, rhs: Square) -> Self::Output {
        BitBoard { value: self.value | rhs.bit() }
    }
}

impl ops::BitOrAssign for BitBoard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.value |= rhs.value;
    }
}

impl ops::BitOrAssign<Square> for BitBoard {
    fn bitor_assign(&mut self, rhs: Square) {
        self.value |= rhs.bit();
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

pub struct BitBoardSubSetsIterator {
    value: u64,
    subset: u64,
    finished: bool
}

impl BitBoardSubSetsIterator {
    fn new(bitboard: BitBoard) -> Self {
        BitBoardSubSetsIterator {
            value: bitboard.value,
            subset: 0,
            finished: false
        }
    }
}

impl Iterator for BitBoardSubSetsIterator {
    type Item = BitBoard;

    fn next(&mut self) -> Option<Self::Item> {
        // https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#All_Subsets_of_any_Set
        if self.finished {
            return None;
        }
        let s = self.subset;
        self.subset = self.subset.wrapping_sub(self.value) & self.value;
        if self.subset == 0 {
            self.finished = true;
        }
        Some(BitBoard::new(s))
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
            for &square in squares.iter() {
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
    fn test_ordinal() {
        let e1 = sq!(e1);
        assert_eq!(e1.ordinal, 4);
        let king_bb = BitBoard::empty().set_all(&[e1]);
        let king = king_bb.single();
        assert_eq!(king.ordinal, 4);
    }

    #[test]
    fn test_accessors() {
        let a3 = sq!(a3);
        let e4 = sq!(e4);
        let h8 = sq!(h8);
        let f5 = sq!(f5);
        let bb = BitBoard::empty().set_all(&[e4, h8]);
        assert_eq!(bb.count(), 2);
        assert_eq!(bb.is_occupied(e4), true);
        assert_eq!(bb.is_occupied(h8), true);
        assert_eq!(bb.is_occupied(f5), false);
        let bb2 = bb.set(f5);
        assert_eq!(bb2.count(), 3);
        assert_eq!(bb2.is_occupied(f5), true);
        assert_eq!(bb2.is_occupied(a3), false);
        let bb3 = bb2.set(a3);
        assert_eq!(bb3.count(), 4);
        assert_eq!(bb3.is_occupied(a3), true);
        let bb4 = bb3.clear(e4);
        assert_eq!(bb4.count(), 3);
        assert_eq!(bb4.is_occupied(e4), false);
    }

    #[test]
    fn test_iter() {
        let a3 = sq!(a3);
        let e4 = sq!(e4);
        let h8 = sq!(h8);
        let f5 = sq!(f5);
        let bb = BitBoard::empty().set_all(&[e4, f5, h8]);
        assert_eq!(&bb.collect::<Vec<Square>>(), &[e4, f5, h8]);
        let bb2 = bb.set(a3).clear(f5);
        assert_eq!(&bb2.collect::<Vec<Square>>(), &[a3, e4, h8]);
    }

    #[test]
    fn test_subsets() {
        let e4 = sq!(e4);
        let f5 = sq!(f5);
        let bb = BitBoard::empty().set_all(&[e4, f5]);
        let ss: Vec<BitBoard> = bb.subsets().collect();
        assert_eq!(ss.len(), 4);
    }
}