use std::io::{BufRead, Write};
use log::{error, info, warn};
use chessica::board::Board;
use crate::search::{Search, TranspositionTable};

pub struct UciSession {
    position: Board,
    tt: TranspositionTable,
    is_running: bool,
    output: Box<dyn Write>
}

const MAX_DEPTH_DEFAULT: usize = 5;
const TT_BITS_DEFAULT: u8 = 24;

impl UciSession {

    pub fn new(output: Box<dyn Write>) -> Self {
        UciSession {
            position: Board::starting_position(),
            tt: TranspositionTable::new(TT_BITS_DEFAULT),
            is_running: true,
            output
        }
    }

    pub fn run(&mut self, input: &mut Box<dyn BufRead>) {
        for line in input.lines() {
            if !self.is_running {
                break;
            }
            match line {
                Ok(command) => { self.handle_command(&command); },
                Err(_) => { break; }
            }
        }
    }

    fn write(&mut self, line: &str) {
        info!(">>> {}", line);
        self.output.write(line.as_bytes()).unwrap();
    }

    fn handle_command(&mut self, command: &String) {
        info!("<<< {}", command);
        let tokens = command.split(" ").collect::<Vec<&str>>();
        match tokens[0] {
            "uci" => {
                self.write("id name Chessica 0.2\n");
                self.write("id author Dan P\n");
                self.write("uciok\n");
            },
            "isready" => {
                self.write("readyok\n");
            },
            "setoption" => {
                self.handle_setoption_command(&tokens[1..]);
            },
            "ucinewgame" => {
                self.handle_ucinewgame_command();
            },
            "position" => {
                self.handle_position_command(&tokens[1..]);
            },
            "go" => {
                self.handle_go_command();
            },
            "stop" => {
                self.handle_stop_command();
            },
            "ponderhit" => {
            },
            "quit" => {
                self.handle_quit_command();
            }
            _ => {
                warn!("Unknown command: {}", tokens[0]);
            }
        }
        self.output.flush().unwrap()
    }

    fn handle_setoption_command(&mut self, _args: &[&str]) {
    }

    fn handle_ucinewgame_command(&mut self) {
        self.position = Board::starting_position();
        self.tt.clear();
    }

    fn handle_position_command(&mut self, args: &[&str]) {
        match args[0] {
            "startpos" => {
                let mut position = Board::starting_position();
                if args.len() > 1 && args[1] == "moves" {
                    for &uci_move in args[2..].iter() {
                        if let Err(_) = position.push_uci(uci_move) {
                            let fen = position.to_fen_string();
                            error!("Illegal move {} in position {}", uci_move, fen);
                            return;
                        }
                    }
                }
                self.position = position;
            },
            "fen" => {
                let fen = args[1..].join(" ");
                match Board::parse_fen(fen.as_str()) {
                    Ok(position) => {
                        self.position = position;
                    }
                    Err(_) => {
                        error!("Failed to parse FEN: {}", fen);
                    }
                }
            },
            _ => {}
        }
    }

    fn handle_go_command(&mut self) {
        let mut search = Search::new(MAX_DEPTH_DEFAULT);
        match search.search(&self.position, &mut self.tt) {
            Some(best_move) => {
                self.write(format!("bestmove {}\n", best_move.to_uci_string()).as_str());
            },
            None => {
                // checkmate/stalemate
            }
        }
    }

    fn handle_stop_command(&mut self) {
    }

    fn handle_quit_command(&mut self) {
        std::process::exit(0);
    }
}
