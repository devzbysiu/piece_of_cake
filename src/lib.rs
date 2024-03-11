#![allow(clippy::missing_errors_doc)]

use log::trace;
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub struct PieceTable<'a> {
    original_buffer: &'a str,
    add_buffer: String,
    pieces: Vec<Piece>,
    undo: Vec<(usize, Piece)>,
}

impl<'a> PieceTable<'a> {
    #[must_use]
    pub fn from_text(txt: &'a str) -> Self {
        Self {
            original_buffer: txt,
            add_buffer: String::new(),
            pieces: vec![Piece::new(0..txt.len(), Source::Original)],
            undo: Vec::new(),
        }
    }

    pub fn insert_char(&mut self, c: char, cursor_idx: usize) {
        let len = self.len();
        if len < cursor_idx {
            panic!("insertion index (is {cursor_idx}) should be <= len (is {len})");
        }

        let start = self.add_buffer().len();
        let add_piece = Piece::new(start..start + 1, Source::Add);

        if cursor_idx == len {
            // we are appending txt at the end
            trace!("text empty or appending at the end");
            self.extend_add_buffer(c);
            self.append_piece(add_piece);
            return;
        }

        trace!("inserting text in the middle");
        let (piece_idx, _) = self.find_piece_idx(cursor_idx);

        self.extend_add_buffer(c);
        let current_piece = self.piece(piece_idx);
        if current_piece.len() > 1 {
            // we need to split the original piece into two and insert new in the middle
            let current_piece = self.remove_piece(piece_idx);
            let (first_piece, second_piece) = current_piece.split_at(cursor_idx);
            self.insert_piece(piece_idx, first_piece);
            self.insert_piece(piece_idx + 1, add_piece);
            self.insert_piece(piece_idx + 2, second_piece);
        } else {
            self.insert_piece(piece_idx, add_piece);
        }
    }

    fn add_buffer(&self) -> &str {
        &self.add_buffer
    }

    fn extend_add_buffer(&mut self, c: char) {
        self.add_buffer.push(c);
    }

    fn original_buffer(&self) -> &str {
        self.original_buffer
    }

    fn append_piece(&mut self, add_piece: Piece) {
        self.pieces.push(add_piece);
    }

    fn find_piece_idx(&self, cursor_idx: usize) -> (usize, usize) {
        let mut txt_len = 0;
        let mut offset = cursor_idx;
        for (idx, piece) in self.pieces.iter().enumerate() {
            if cursor_idx < txt_len + piece.len() {
                return (idx, offset);
            }
            offset -= piece.len();
            txt_len += piece.len();
        }
        panic!("cursor index is out of range")
    }

    fn remove_piece(&mut self, idx: usize) -> Piece {
        assert!(idx < self.pieces.len());
        self.pieces.remove(idx)
    }

    fn insert_piece(&mut self, current_idx: usize, first_piece: Piece) {
        self.pieces.insert(current_idx, first_piece);
    }

    pub fn remove_char(&mut self, cursor_idx: usize) -> Option<char> {
        let char = self.char_at(cursor_idx);
        let (piece_idx, offset) = self.find_piece_idx(cursor_idx);
        let current_piece = self.remove_piece(piece_idx);
        let real_idx = current_piece.range.start + offset;
        if current_piece.range.start < real_idx && real_idx < current_piece.range.end - 1 {
            let (first_piece, mut second_piece) = current_piece.split_at(cursor_idx);
            second_piece.range.start += 1;
            self.insert_piece(piece_idx, first_piece);
            self.insert_piece(piece_idx + 1, second_piece);
        } else if current_piece.range.start == real_idx {
            let mut current_piece = current_piece;
            current_piece.range.start += 1;
            self.insert_piece(piece_idx, current_piece);
        } else {
            let mut current_piece = current_piece;
            current_piece.range.end -= 1;
            self.insert_piece(piece_idx, current_piece);
        }
        Some(char)
    }

    pub fn remove(&mut self, range: Range<usize>) -> Option<String> {
        let mut chars = vec![' '; range.len()];
        let mut i = range.len() - 1;
        for cursor_idx in range.rev() {
            chars[i] = self.remove_char(cursor_idx)?;
            i = i.saturating_sub(1);
        }
        Some(chars.into_iter().collect())
    }

    pub fn undo(&mut self) {
        let last_idx = self.pieces.len() - 1;
        let last_piece = self.pieces.remove(last_idx);
        self.undo.push((last_idx, last_piece));
    }

    pub fn redo(&mut self) {
        let (last_op_idx, last_op) = self.undo.remove(self.undo.len() - 1);
        self.pieces.insert(last_op_idx, last_op);
    }

    #[must_use]
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

    fn append_from(&self, txt: &mut String, piece: &Piece) {
        let buff = match piece.source {
            Source::Original => &self.original_buffer[piece.range.clone()],
            Source::Add => &self.add_buffer[piece.range.clone()],
        };
        txt.push_str(buff);
    }

    pub fn len(&self) -> usize {
        if self.pieces.is_empty() {
            return self.original_buffer().len();
        }
        let mut len = 0;
        for piece in &self.pieces {
            len += piece.range.len();
        }
        len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn piece(&self, current_piece_idx: usize) -> &Piece {
        &self.pieces[current_piece_idx]
    }

    fn char_at(&self, char_idx: usize) -> char {
        let (piece_idx, offset) = self.find_piece_idx(char_idx);
        let piece = self.piece(piece_idx);
        let buff = match piece.source {
            Source::Original => self.original_buffer(),
            Source::Add => self.add_buffer(),
        };
        buff.chars().nth(piece.range.start + offset).unwrap()
    }
}

impl<'a> Default for PieceTable<'a> {
    fn default() -> Self {
        Self::from_text("")
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Piece {
    range: Range<usize>,
    source: Source,
}

impl Piece {
    fn new(range: Range<usize>, source: Source) -> Self {
        Self { range, source }
    }

    fn split_at(self, idx: usize) -> (Piece, Piece) {
        let mut first_piece = self.clone();
        let mut second_piece = self.clone();
        first_piece.range.end = idx;
        second_piece.range.start = idx;
        (first_piece, second_piece)
    }

    fn len(&self) -> usize {
        self.range.len()
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

    fn init_logger() {
        let _ = env_logger::try_init();
    }

    mod insert_char {
        use super::*;

        #[test]
        fn should_add_piece_at_the_beginning() {
            init_logger();
            // given
            let mut table = PieceTable::default();
            let new_char = 's';
            let cursor = 0;

            // when
            table.insert_char(new_char, cursor);

            // then
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0..0, Source::Original),
                    Piece::new(0..1, Source::Add),
                ]
            );
        }

        #[test]
        fn should_add_each_char_piece() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("a");
            let cursor = 1;

            // when
            table.insert_char('b', cursor);
            table.insert_char('c', cursor);

            // then
            assert_eq!(table.pieces.len(), 3);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0..1, Source::Original),
                    Piece::new(1..2, Source::Add),
                    Piece::new(0..1, Source::Add),
                ]
            );
        }

        #[test]
        fn should_add_char_when_cursor_moved_back() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("a");
            let cursor = 1;
            table.insert_char('b', cursor);
            table.insert_char('c', cursor + 1);

            // when

            // then
            assert_eq!(table.pieces.len(), 3);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0..1, Source::Original),
                    Piece::new(0..1, Source::Add),
                    Piece::new(1..2, Source::Add),
                ]
            );
        }

        #[test]
        fn should_add_line_piece_appended_at_the_end() {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_char = 's';
            let cursor = initial_txt.len();

            // when
            table.insert_char(new_char, cursor);

            // then
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0..initial_txt.len(), Source::Original),
                    Piece::new(0..1, Source::Add),
                ]
            );
        }

        #[test]
        fn should_add_line_inserted_in_the_middle() {
            init_logger();
            // given
            let txt_before = "some ";
            let txt_after = "initial text";
            let initial_txt = &format!("{txt_before}{txt_after}");
            let mut table = PieceTable::from_text(initial_txt);
            let new_char = 's';
            let cursor = 5;

            // when
            table.insert_char(new_char, cursor);

            // then
            assert_eq!(table.pieces.len(), 3);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0..txt_before.len(), Source::Original),
                    Piece::new(0..1, Source::Add),
                    Piece::new(
                        txt_before.len()..txt_before.len() + txt_after.len(),
                        Source::Original
                    ),
                ]
            );
        }
    }

    mod remove_char {
        use super::*;

        #[test]
        fn should_remove_char_from_the_middle() {
            init_logger();
            // given
            let txt_before = "initial";
            let txt_after = "txt";
            let initial_txt = &format!("{txt_before} {txt_after}");
            let mut table = PieceTable::from_text(initial_txt);

            // when
            let removed = table.remove_char(7);

            // then
            assert_eq!(removed, Some(' '));
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0..txt_before.len(), Source::Original),
                    Piece::new(
                        (txt_before.len() + 1)..txt_before.len() + 1 + txt_after.len(),
                        Source::Original
                    ),
                ]
            );
        }

        #[test]
        fn should_remove_end_char_multiple_times() {
            init_logger();
            // given
            let initial_text = "initial text";
            let mut table = PieceTable::from_text(initial_text);
            let remove_count = 5;

            // when
            let mut removed_chars = Vec::new();
            for i in 0..remove_count {
                let removed = table.remove_char(11 - i);
                removed_chars.push(removed.unwrap());
            }

            // then
            assert_eq!(removed_chars, ['t', 'x', 'e', 't', ' ']);
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(
                table.pieces,
                [Piece::new(
                    0..initial_text.len() - remove_count,
                    Source::Original
                )]
            );
        }

        #[test]
        fn should_delete_consecutive_chars() {
            init_logger();
            // given
            let initial_text = "initial text";
            let mut table = PieceTable::from_text(initial_text);
            let remove_count = 5;

            // when
            let mut removed_chars = Vec::new();
            for _ in 0..remove_count {
                let removed = table.remove_char(7);
                removed_chars.push(removed.unwrap());
            }

            // then
            assert_eq!(removed_chars, [' ', 't', 'e', 'x', 't']);
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0..7, Source::Original),
                    // TODO: Can I remove piece with no characters? (empty range)
                    Piece::new(initial_text.len()..initial_text.len(), Source::Original),
                ]
            );
        }

        #[test]
        fn should_remove_chars_at_the_end() {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);

            // when
            let removed1 = table.remove_char(initial_txt.len() - 1);
            let removed2 = table.remove_char(initial_txt.len() - 2);

            // then
            assert_eq!(removed1, Some('t'));
            assert_eq!(removed2, Some('x'));
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(
                table.pieces,
                [Piece::new(0..(initial_txt.len() - 2), Source::Original)]
            );
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn should_remove_range() {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);

            // when
            let removed = table.remove(7..12);

            // then
            assert_eq!(removed, Some(" text".to_string()));
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(table.pieces, [Piece::new(0..7, Source::Original)]);
        }
    }

    mod undo {
        use super::*;

        #[test]
        fn shuld_undo_last_operation() {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_char = 's';
            table.insert_char(new_char, initial_txt.len());
            assert_eq!(table.pieces.len(), 2);
            assert!(table.undo.is_empty());

            // when
            table.undo();

            // then
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(table.undo.len(), 1);
        }
    }

    mod redo {
        use super::*;

        #[test]
        fn shuld_redo_last_operation() {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_char = 's';
            table.insert_char(new_char, initial_txt.len());
            table.undo();
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(table.undo.len(), 1);

            // when
            table.redo();

            // then
            assert_eq!(table.pieces.len(), 2);
            assert!(table.undo.is_empty());
        }
    }

    mod project {
        use super::*;

        #[test]
        fn empty_table_projects_empty_string() {
            init_logger();
            // given
            let table = PieceTable::default();

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, String::new());
        }

        #[test]
        fn should_show_added_line_when_table_is_empty() {
            init_logger();
            // given
            let mut table = PieceTable::default();
            let new_char = 's';
            let cursor = 0;
            table.insert_char(new_char, cursor);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, char::to_string(&new_char));
        }

        #[test]
        fn should_show_added_chars() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("a");
            let cursor = 1;
            table.insert_char('b', cursor);
            table.insert_char('c', cursor);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "acb");
        }

        #[test]
        fn should_show_line_appended_at_the_end() {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_char = 's';
            let cursor = initial_txt.len();
            table.insert_char(new_char, cursor);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, format!("{initial_txt}{new_char}"));
        }

        #[test]
        fn should_show_line_inserted_in_the_middle() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("some initial text");
            let new_char = 's';
            let cursor = 5;
            table.insert_char(new_char, cursor);

            // when
            let txt = table.project();

            // then
            assert_eq!(&txt, "some sinitial text");
        }

        #[test]
        fn should_remove_char_from_the_middle() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove_char(7);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initialtext");
        }

        #[test]
        fn should_remove_end_char_multiple_times() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove_char(11);
            table.remove_char(10);
            table.remove_char(9);
            table.remove_char(8);
            table.remove_char(7);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial");
        }

        #[test]
        fn should_delete_consecutive_chars() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove_char(7);
            table.remove_char(7);
            table.remove_char(7);
            table.remove_char(7);
            table.remove_char(7);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial");
        }

        #[test]
        fn should_remove_chars_at_the_end() {
            init_logger();
            // given
            let initial_text = "initial text";
            let mut table = PieceTable::from_text(initial_text);
            table.remove_char(initial_text.len() - 1);
            table.remove_char(initial_text.len() - 2);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial te");
        }

        #[test]
        fn should_not_show_removed_range() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove(7..12);

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial");
        }
    }

    mod len_and_empty {
        use super::*;

        #[test]
        fn empty_piece_table_has_len_zero() {
            init_logger();
            // given
            let piece_table = PieceTable::from_text("");

            // when
            let len = piece_table.len();

            // then
            assert_eq!(len, 0);
        }

        #[test]
        fn piece_table_from_text_has_len_equal_to_initial_text() {
            init_logger();
            // given
            let initial_txt = "initial text";
            let piece_table = PieceTable::from_text(initial_txt);

            // when
            let len = piece_table.len();

            // then
            assert_eq!(len, initial_txt.len());
        }

        #[test]
        fn len_takes_into_account_modifiec_piece_table() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("a");
            let cursor = 1;
            table.insert_char('b', cursor);
            table.insert_char('c', cursor);

            // when
            let len = table.len();

            // then
            assert_eq!(len, 3);
        }

        #[test]
        fn empty_piece_table_is_empty() {
            init_logger();
            // given
            let piece_table = PieceTable::from_text("");

            // when
            let is_empty = piece_table.is_empty();

            // then
            assert!(is_empty);
        }

        #[test]
        fn piece_table_from_text_is_not_empty() {
            init_logger();
            // given
            let piece_table = PieceTable::from_text("initial text");

            // when
            let is_empty = piece_table.is_empty();

            // then
            assert!(!is_empty);
        }
    }
}
