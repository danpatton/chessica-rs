#[derive(Debug)]
pub struct IllegalMoveError;

#[derive(Debug)]
pub struct FenParseError;

#[derive(Debug)]
pub struct FenCharParseError;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseSquareError;

#[derive(Debug)]
pub struct MagicTableMaxAttemptsExceededError;

#[derive(Debug)]
pub struct HashCollisionError;
