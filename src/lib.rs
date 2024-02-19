#![allow(unused)]
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct PieceTable {
    buffers: HashMap<Source, String>,
    pieces: Vec<Piece>,
}

impl PieceTable {
    pub fn from_text(txt: String) -> Self {
        let mut buffers = HashMap::new();
        let txt_len = txt.len();
        buffers.insert(Source::Original, txt);
        buffers.insert(Source::Add, String::new());
        Self {
            buffers,
            pieces: vec![Piece::new(0, txt_len, Source::Original)],
        }
    }

    pub fn add(&mut self, txt: String, cursor_idx: usize) {
        let start = self.add_buffer().len();
        let add_piece = Piece::new(start, start + txt.len(), Source::Add);

        self.buffers
            .entry(Source::Add)
            .and_modify(|b| b.push_str(&txt));

        if self.original_buffer().is_empty() || self.original_buffer().len() == cursor_idx {
            // either text is empty or we are appending to it
            self.pieces.push(add_piece);
        } else {
            // we need to split the original piece into two and insert new in the middle
            let current_idx = self.find_current_piece_idx(cursor_idx);
            let mut first_piece = self.pieces.remove(current_idx);
            let mut second_piece = first_piece.clone();

            first_piece.end = cursor_idx;
            second_piece.start = cursor_idx;

            self.pieces.insert(current_idx, first_piece);
            self.pieces.insert(current_idx + 1, add_piece);
            self.pieces.insert(current_idx + 2, second_piece);
        }
    }

    pub fn remove(&mut self, txt: String, cursor_idx: usize) {
        todo!()
    }

    pub fn project(&self) -> String {
        if self.pieces.is_empty() {
            return self.original_buffer().to_string();
        }
        let mut txt = String::new();
        for piece in &self.pieces {
            self.append_from(&mut txt, piece);
        }
        txt
    }

    fn original_buffer(&self) -> &String {
        self.buffers
            .get(&Source::Original)
            .expect("original buffer")
    }

    fn add_buffer(&self) -> &String {
        self.buffers.get(&Source::Add).expect("add buffer")
    }

    fn append_from(&self, txt: &mut String, piece: &Piece) {
        self.buffers
            .get(&piece.source)
            .map(|t| txt.push_str(&t[piece.start..piece.end]));
    }

    fn find_current_piece_idx(&self, cursor_idx: usize) -> usize {
        self.pieces
            .iter()
            .enumerate()
            .find(|(idx, p)| p.start <= cursor_idx && p.start < p.end)
            .expect("current piece")
            .0
    }
}

impl Default for PieceTable {
    fn default() -> Self {
        let mut buffers = HashMap::new();
        buffers.insert(Source::Original, String::new());
        buffers.insert(Source::Add, String::new());
        Self {
            buffers,
            pieces: vec![Piece::new(0, 0, Source::Original)],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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
                    pieces: vec![
                        Piece::new(0, 0, Source::Original),
                        Piece::new(0, new_line.len(), Source::Add)
                    ],
                }
            );
        }
    }

    mod project {
        use super::*;

        #[test]
        fn should_show_added_line() {
            // given
            let mut table = PieceTable::default();
            let new_line: String = "some line".into();
            let cursor = 0;
            table.add(new_line.clone(), cursor);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, new_line);
        }

        #[test]
        fn should_show_line_inserted_in_the_middle() {
            // given
            let mut table = PieceTable::from_text("some initial text".into());
            let new_line: String = "some line ".into();
            let cursor = 5;
            table.add(new_line.clone(), cursor);

            // when
            let txt = table.project();

            // then
            assert_eq!(&txt, "some some line initial text");
        }
    }
}
