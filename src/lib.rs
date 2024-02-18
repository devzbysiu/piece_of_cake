#![allow(unused)]
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct PieceTable {
    buffers: HashMap<Source, String>,
    pieces: Vec<Piece>,
}

impl PieceTable {
    pub fn add(&mut self, txt: String, cursor_idx: usize) {
        let piece = Piece::new(cursor_idx, cursor_idx + txt.len(), Source::Add);
        self.pieces.push(piece);
        self.buffers
            .entry(Source::Add)
            .and_modify(|buff| buff.push_str(&txt));
    }

    pub fn remove(&mut self, txt: String, cursor_idx: usize) {
        todo!()
    }

    pub fn project(&self) -> String {
        todo!()
    }
}

impl Default for PieceTable {
    fn default() -> Self {
        let mut txt = HashMap::new();
        txt.insert(Source::Original, String::new());
        txt.insert(Source::Add, String::new());
        Self {
            buffers: txt,
            pieces: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Piece {
    start: usize,
    end: usize,
    source: Source,
}

impl Piece {
    fn new(start: usize, end: usize, source: Source) -> Self {
        Self { start, end, source }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Source {
    Original,
    Add,
}

#[cfg(test)]
mod tests {
    use super::*;

    use maplit::hashmap;

    mod add {
        use super::*;

        use crate::PieceTable;

        #[test]
        fn should_add_to_empty_table() {
            // given
            let mut table = PieceTable::default();
            let new_line: String = "some line".into();

            // when
            table.add(new_line.clone(), 0);

            // then
            assert_eq!(
                table,
                PieceTable {
                    buffers: hashmap! {
                        Source::Original => "".into(),
                        Source::Add => new_line.clone(),
                    },
                    pieces: vec![Piece::new(0, new_line.len(), Source::Add)],
                }
            );
        }
    }
}
