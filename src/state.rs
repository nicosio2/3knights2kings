use crate::encoding::{PackedState, SmallState};
use std::cmp::Ordering;
use shakmaty::fen::Fen;
use shakmaty::Color;
use shakmaty::Square;
use std::cmp::min;
use std::cmp::max;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

impl Position {
    pub fn from_u8(i: u8) -> Self {
        Position { x: i & 0b111, y: (i >> 3) & 0b111 }
    }
    pub fn to_u8(&self) -> u8 {
        self.x | (self.y << 3)
    }

    pub fn from_u8_rim(i: u8) -> Self {
        assert!(i < 28);
        if i < 8 {
            Position { x: i, y: 0 }
        } else if i < 16 {
            Position { x: i - 8, y: 7 }
        } else if i < 22 {
            Position { x: 0, y: i - 15 }
        } else {
            Position { x: 7, y: i - 21 }
        }
    }
    pub fn to_u8_rim(self) -> u8 {
        assert!(self.is_on_rim());
        if self.y == 0 {
            self.x
        } else if self.y == 7 {
            self.x + 8
        } else if self.x == 0 {
            self.y + 15
        } else {
            self.y + 21
        }
    }

    pub fn is_on_rim(self) -> bool {
        self.x == 0 || self.y == 0 || self.x == 7 || self.y == 7
    }

    pub fn from_u8_bottom_left(i: u8) -> Self {
        assert!(i < 16);
        Position { x: i & 0b11, y: i >> 2 }
    }
    pub fn to_u8_bottom_left(&self) -> u8 {
        assert!(self.x < 4 && self.y < 4);
        self.x + (self.y << 2)
    }

    pub fn rotate_clockwise(self) -> Self {
        Position { x: self.y, y: 7 - self.x }
    }
    pub fn rotate_counterclockwise(self) -> Self {
        Position { x: 7 - self.y, y: self.x }
    }
    pub fn rotate_twice(self) -> Self {
        Position { x: 7 - self.x, y: 7 - self.y }
    }

    pub fn is_out_of_bounds(&self, dx: i16, dy: i16) -> bool {
        self.x as i16 + dx > 7 || self.x as i16 + dx < 0 || self.y as i16 + dy > 7 || self.y as i16 + dy < 0
    }

    pub fn add(&self, dx: i16, dy: i16) -> Self {
        Position {
            x: (self.x as i16 + dx) as u8,
            y: (self.y as i16 + dy) as u8,
        }
    }

    pub fn from_square(square: Square) -> Self {
        let (x, y) = square.coords();
        Position {x: u8::from(x), y: u8::from(y)}
    }

    pub fn from_string(s: &String) -> Result<Self, String> {
        if s.len() != 2 {
            Err(format!("{} is not a valid square (has to have length 2)!", s))
        }
        else {
            let file = s.chars().nth(0).unwrap().to_ascii_lowercase();
            let rank = s.chars().nth(1).unwrap().to_ascii_lowercase();
            if file < 'a' || file > 'h' {
                Err(format!("{} is not a valid square (file has to be between 'a' and 'h')!", s))
            }
            else if rank < '1' || rank > '8' {
                Err(format!("{} is not a valid square (rank has to be between 1 and 8)!", s))
            }
            else {
                Ok(Position { x: file as u8 - 'a' as u8, y: rank as u8 - '1' as u8 })
            }
        }
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Position) -> Ordering {
        self.to_u8().cmp(&(*other).to_u8())
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Position) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct State {
    pub white_king: Position,
    pub knights: [Position; 3],
    pub black_king: Position,
    pub target_field: Position,
    pub white_to_move: bool,
}

impl State {
    pub fn unpack(packed: PackedState) -> Self {
        let SmallState {
            white_king: white_king_packed,
            knights: knights_packed,
            black_king: black_king_packed,
            target_field: target_field_packed,
            white_to_move: white_to_move_packed,
        } = SmallState::decode(packed);

        let white_king = Position::from_u8_bottom_left(white_king_packed);
        let black_king = black_king_packed + if white_king.to_u8() <= black_king_packed { 1 } else { 0 };
        let mut knights = [0u8; 3];

        let mut prev: Vec<u8> = vec![white_king.to_u8(), black_king];
        for i in 0..3 {
            knights[i] = knights_packed[i];
            prev.sort_unstable();
            for j in &prev {
                if *j <= knights[i] {
                    knights[i] += 1;
                }
            }
            prev.push(knights[i]);
        }

        let black_king = Position::from_u8(black_king);
        let knights = [Position::from_u8(knights[0]), Position::from_u8(knights[1]), Position::from_u8(knights[2])];
        let target_field = Position::from_u8_rim(target_field_packed);
        let white_to_move = white_to_move_packed == 1;

        State {
            white_king,
            knights,
            black_king,
            white_to_move,
            target_field,
        }
    }

    pub fn apply_to_positions(&self, f: &Fn(Position) -> Position) -> Self {
        State { white_king: f(self.white_king), knights: [f(self.knights[0]), f(self.knights[1]), f(self.knights[2])], black_king: f(self.black_king), target_field: f(self.target_field), ..*self }
    }

    pub fn rotate_clockwise(&self) -> Self {
        self.apply_to_positions(&Position::rotate_clockwise)
    }

    pub fn rotate_counterclockwise(&self) -> Self {
        self.apply_to_positions(&Position::rotate_counterclockwise)
    }

    pub fn rotate_twice(&self) -> Self {
        self.apply_to_positions(&Position::rotate_twice)
    }

    pub fn sort_knights(&self) -> Self {
        let min_knight = min(self.knights[0], min(self.knights[1], self.knights[2]));
        let max_knight = max(self.knights[0], max(self.knights[1], self.knights[2]));
        let middle_knight = Position::from_u8(self.knights[0].to_u8() ^ self.knights[1].to_u8() ^ self.knights[2].to_u8() ^ max_knight.to_u8() ^ min_knight.to_u8());

        assert!(min_knight < middle_knight && middle_knight < max_knight);
        State { knights: [min_knight, middle_knight, max_knight], ..*self }
    }

    pub fn normalize(&self) -> State {
        (if self.white_king.x >= 4 && self.white_king.y < 4 {
            // lower right
            self.rotate_clockwise()
        } else if self.white_king.x < 4 && self.white_king.y >= 4 {
            // upper left
            self.rotate_counterclockwise()
        } else if self.white_king.x >= 4 && self.white_king.y >= 4 {
            // upper right
            self.rotate_twice()
        } else {
            // lower left
            *self
        }).sort_knights()
    }

    fn pack_normalized(&self) -> PackedState {
        let State {
            white_king,
            black_king,
            knights,
            white_to_move,
            target_field,
        } = self;
        let black_king = black_king.to_u8() - if white_king < black_king { 1 } else { 0 };

        let mut knights_packed = [knights[0].to_u8(), knights[1].to_u8(), knights[2].to_u8()];

        for i in 0..3 {
            if white_king.to_u8() < knights_packed[i] {
                knights_packed[i] -= 1;
            }
            if black_king < knights_packed[i] {
                knights_packed[i] -= 1;
            }
            knights_packed[i] -= i as u8;
        }

        let white_to_move = if self.white_to_move { 1u8 } else { 0u8 };

        SmallState {
            white_king: white_king.to_u8_bottom_left(),
            black_king,
            knights: knights_packed,
            target_field: target_field.to_u8_rim(),
            white_to_move,
        }
            .encode()
    }

    pub fn pack(&self) -> PackedState {
        self.normalize().pack_normalized()
    }

    pub fn to_fen(self) -> String {
        let s = self.normalize();
        let mut result = String::from("");
        let mut position = [""; 64];
        position[s.white_king.to_u8() as usize] = "K";
        position[s.black_king.to_u8() as usize] = "k";
        for knight in &s.knights {
            position[knight.to_u8() as usize] = "N";
        }
        let mut counter = 0;
        for i in 0..8 {
            for j in 0..8 {
                if position[(7 - i) * 8 + j] == "" {
                    counter += 1;
                } else {
                    if counter > 0 {
                        result += &counter.to_string();
                        counter = 0;
                    }
                    result += position[(7 - i) * 8 + j];
                }
            }
            if counter != 0 {
                result += &counter.to_string();
                counter = 0;
            }
            result += "/";
        }
        result + " " + if s.white_to_move { "w" } else { "b" } + " - - 0 1"
    }

    pub fn from_fen(s: &String, target: Position) -> Result<Self, String> {
        let fen = s.parse::<Fen>().unwrap();

        let black_king_opt = fen.board.king_of(Color::Black);
        let white_king_opt = fen.board.king_of(Color::White);

        if let None = black_king_opt {
            Err(String::from("No black king found!"))
        }
        else if let None = white_king_opt {
            Err(String::from("No white king found!"))
        }
        else if (fen.board.white() & fen.board.knights()).count() != 3 {
            Err(String::from("Wrong amount of white knighs!"))
        }
        else if fen.board.white().count() != 4 {
            Err(String::from("Wrong amount of white pieces!"))
        }
        else if fen.board.black().count() != 1 {
            Err(String::from("Wrong amount of black pieces!"))
        }
        else {
            let black_king = Position::from_square(black_king_opt.unwrap());
            let white_king = Position::from_square(white_king_opt.unwrap());
            let mut i = 0;
            let mut knights = [Position::from_u8(0), Position::from_u8(0), Position::from_u8(0)];
            for square in fen.board.knights().into_iter() {
                if fen.board.color_at(square).unwrap() == Color::White {
                    knights[i] = Position::from_square(square);
                    i += 1;
                }
            };
            Ok(State {
                white_king,
                black_king,
                knights,
                white_to_move: fen.turn == Color::White,
                target_field: target
            })
        }
    }

    pub fn to_lichess(self) -> String {
        String::from("https://lichess.org/editor/") + &self.to_fen().replace(" ", "_")
    }
}

impl PartialEq for State {
    fn eq(&self, other: &State) -> bool {
        let normalized = self.normalize();
        let o_normalized = other.normalize();
        (normalized.white_king, normalized.black_king, normalized.knights, normalized.target_field, normalized.white_to_move) == (o_normalized.white_king, o_normalized.black_king, o_normalized.knights, o_normalized.target_field, o_normalized.white_to_move)
    }
}
