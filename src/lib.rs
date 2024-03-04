#![allow(clippy::missing_errors_doc)]

use log::{error, trace};
use std::ops::Range;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ModificationError {
    #[error("modification falls outside of the available text")]
    OutOfRange,
}

pub type Result<T> = std::result::Result<T, ModificationError>;

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
            pieces: vec![Piece::new(0, txt.len(), Source::Original)],
            undo: Vec::new(),
        }
    }

    pub fn add(&mut self, txt: &str, cursor_idx: usize) -> Result<()> {
        let start = self.add_buffer().len();
        let add_piece = Piece::new(start, start + txt.len(), Source::Add);

        if self.original_buffer().is_empty() || self.original_buffer().len() == cursor_idx {
            // either text is empty or we are appending to it
            trace!("text empty or appending at the end");
            self.extend_add_buffer(txt);
            self.add_piece(add_piece);
            return Ok(());
        }

        // we need to split the original piece into two and insert new in the middle
        trace!("inserting text in the middle");
        let Some(current_piece_idx) = self.find_current_piece_idx(cursor_idx) else {
            error!("cursor is outside of the text");
            return Err(ModificationError::OutOfRange);
        };
        self.extend_add_buffer(txt);
        let first_piece = self.remove_piece(current_piece_idx);
        let (first_piece, second_piece) = first_piece.split_at(cursor_idx);
        self.insert_piece(current_piece_idx, first_piece);
        self.insert_piece(current_piece_idx + 1, add_piece);
        self.insert_piece(current_piece_idx + 2, second_piece);
        Ok(())
    }

    fn add_buffer(&self) -> &str {
        &self.add_buffer
    }

    fn extend_add_buffer(&mut self, txt: &str) {
        self.add_buffer.push_str(txt);
    }

    fn original_buffer(&self) -> &str {
        self.original_buffer
    }

    fn add_piece(&mut self, add_piece: Piece) {
        self.pieces.push(add_piece);
    }

    fn find_current_piece_idx(&self, cursor_idx: usize) -> Option<usize> {
        self.pieces
            .iter()
            .enumerate()
            .find(|(_, p)| p.start <= cursor_idx && cursor_idx < p.end)
            .map(|entry| entry.0)
    }

    fn remove_piece(&mut self, idx: usize) -> Piece {
        assert!(idx < self.pieces.len());
        self.pieces.remove(idx)
    }

    fn insert_piece(&mut self, current_idx: usize, first_piece: Piece) {
        self.pieces.insert(current_idx, first_piece);
    }

    pub fn remove_char(&mut self, cursor_idx: usize) -> Result<()> {
        let Some(current_piece_idx) = self.find_current_piece_idx(cursor_idx) else {
            error!("cursor outside of the text");
            return Err(ModificationError::OutOfRange);
        };
        let mut current_piece = self.remove_piece(current_piece_idx);
        trace!("current piece idx for cursor {cursor_idx}: {current_piece_idx:?}");
        trace!("current piece {current_piece:#?}");
        if current_piece.start < cursor_idx && cursor_idx < current_piece.end - 1 {
            trace!("modifying in the middle");
            trace!("splitting at {cursor_idx}");
            trace!("before {current_piece:#?}");
            let (first_piece, mut second_piece) = current_piece.split_at(cursor_idx);
            trace!("after {first_piece:#?} & {second_piece:#?}");
            trace!("before: {second_piece:#?}");
            second_piece.start += 1;
            trace!("after: {second_piece:#?}");
            self.insert_piece(current_piece_idx, first_piece);
            self.insert_piece(current_piece_idx + 1, second_piece);
        } else if current_piece.start == cursor_idx {
            trace!("modifying start");
            trace!("before: {current_piece:#?}");
            current_piece.start += 1;
            trace!("after: {current_piece:#?}");
            self.insert_piece(current_piece_idx, current_piece);
        } else if current_piece.end - 1 == cursor_idx {
            trace!("modifying end");
            trace!("before: {current_piece:#?}");
            current_piece.end -= 1;
            trace!("after: {current_piece:#?}");
            self.insert_piece(current_piece_idx, current_piece);
        }
        Ok(())
    }

    pub fn remove(&mut self, range: Range<usize>) -> Result<()> {
        for cursor_idx in range.rev() {
            self.remove_char(cursor_idx)?;
        }
        Ok(())
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
            Source::Original => &self.original_buffer[piece.start..piece.end],
            Source::Add => &self.add_buffer[piece.start..piece.end],
        };
        txt.push_str(buff);
    }

    pub fn len(&self) -> usize {
        if self.pieces.is_empty() {
            return self.original_buffer().len();
        }
        let mut len = 0;
        for piece in &self.pieces {
            len += piece.end - piece.start;
        }
        len
    }
}

impl<'a> Default for PieceTable<'a> {
    fn default() -> Self {
        Self::from_text("")
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
        assert!(start <= end);
        Self { start, end, source }
    }

    fn split_at(self, idx: usize) -> (Piece, Piece) {
        let mut first_piece = self.clone();
        let mut second_piece = self.clone();
        first_piece.end = idx;
        second_piece.start = idx;
        (first_piece, second_piece)
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

    mod add {
        use super::*;

        #[test]
        fn should_add_piece_at_the_beginning() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::default();
            let new_line = "some line";
            let cursor = 0;

            // when
            table.add(new_line, cursor)?;

            // then
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0, 0, Source::Original),
                    Piece::new(0, new_line.len(), Source::Add),
                ]
            );

            Ok(())
        }

        #[test]
        fn should_add_each_char_piece() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("a");
            let cursor = 1;

            // when
            table.add("b", cursor)?;
            table.add("c", cursor)?;

            // then
            assert_eq!(table.pieces.len(), 3);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0, 1, Source::Original),
                    Piece::new(0, 1, Source::Add),
                    Piece::new(1, 2, Source::Add),
                ]
            );

            Ok(())
        }

        #[test]
        fn should_add_line_piece_appended_at_the_end() -> Result<()> {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_line = " some line";
            let cursor = initial_txt.len();

            // when
            table.add(new_line, cursor)?;

            // then
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0, initial_txt.len(), Source::Original),
                    Piece::new(0, new_line.len(), Source::Add),
                ]
            );

            Ok(())
        }

        #[test]
        fn should_add_line_inserted_in_the_middle() -> Result<()> {
            init_logger();
            // given
            let txt_before = "some ";
            let txt_after = "initial text";
            let initial_txt = &format!("{txt_before}{txt_after}");
            let mut table = PieceTable::from_text(initial_txt);
            let new_line = "some line ";
            let cursor = 5;

            // when
            table.add(new_line, cursor)?;

            // then
            assert_eq!(table.pieces.len(), 3);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0, txt_before.len(), Source::Original),
                    Piece::new(0, new_line.len(), Source::Add),
                    Piece::new(
                        txt_before.len(),
                        txt_before.len() + txt_after.len(),
                        Source::Original
                    ),
                ]
            );

            Ok(())
        }

        #[test]
        fn should_return_out_of_range_error_when_adding_at_wrong_cursor() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("some text");
            let new_line = "some line";
            let wrong_cursor = usize::MAX;

            // when
            let res = table.add(new_line, wrong_cursor);

            // then
            assert_eq!(res, Err(ModificationError::OutOfRange));
        }
    }

    mod remove_char {
        use super::*;

        #[test]
        fn should_remove_char_from_the_middle() -> Result<()> {
            init_logger();
            // given
            let txt_before = "initial";
            let txt_after = "txt";
            let initial_txt = &format!("{txt_before} {txt_after}");
            let mut table = PieceTable::from_text(initial_txt);

            // when
            table.remove_char(7)?;

            // then
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0, txt_before.len(), Source::Original),
                    Piece::new(
                        txt_before.len() + 1,
                        txt_before.len() + 1 + txt_after.len(),
                        Source::Original
                    ),
                ]
            );

            Ok(())
        }

        #[test]
        fn should_remove_end_char_multiple_times() -> Result<()> {
            init_logger();
            // given
            let initial_text = "initial text";
            let mut table = PieceTable::from_text(initial_text);
            let remove_count = 5;

            // when
            for i in 0..remove_count {
                table.remove_char(11 - i)?;
            }

            // then
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(
                table.pieces,
                [Piece::new(
                    0,
                    initial_text.len() - remove_count,
                    Source::Original
                )]
            );

            Ok(())
        }

        #[test]
        fn should_delete_consecutive_chars() -> Result<()> {
            init_logger();
            // given
            let initial_text = "initial text";
            let mut table = PieceTable::from_text(initial_text);
            let remove_count = 5;

            // when
            for i in 0..remove_count {
                table.remove_char(7 + i)?;
            }

            // then
            assert_eq!(table.pieces.len(), 2);
            assert_eq!(
                table.pieces,
                [
                    Piece::new(0, 7, Source::Original),
                    // TODO: Can I remove piece with no characters? (empty range)
                    Piece::new(initial_text.len(), initial_text.len(), Source::Original),
                ]
            );

            Ok(())
        }

        #[test]
        fn should_remove_chars_at_the_end() -> Result<()> {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);

            // when
            table.remove_char(initial_txt.len() - 1)?;
            table.remove_char(initial_txt.len() - 2)?;

            // then
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(
                table.pieces,
                [Piece::new(0, initial_txt.len() - 2, Source::Original)]
            );

            Ok(())
        }

        #[test]
        fn should_return_out_of_range_error_when_removing_char_at_wrong_cursor() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("some text");
            let wrong_cursor = usize::MAX;

            // when
            let res = table.remove_char(wrong_cursor);

            // then
            assert_eq!(res, Err(ModificationError::OutOfRange));
        }

        #[test]
        fn should_return_out_of_range_error_when_removing_char_just_after_available_range() {
            init_logger();
            // given
            let text = "some text";
            let mut table = PieceTable::from_text(text);
            let wrong_cursor = text.len();

            // when
            let res = table.remove_char(wrong_cursor);

            // then
            assert_eq!(res, Err(ModificationError::OutOfRange));
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn should_remove_range() -> Result<()> {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);

            // when
            table.remove(7..12)?;

            // then
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(table.pieces, [Piece::new(0, 7, Source::Original)]);

            Ok(())
        }

        #[test]
        fn should_return_out_of_range_error_when_removing_at_wrong_cursor() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");

            // when
            let res = table.remove(7..150);

            // then
            assert_eq!(res, Err(ModificationError::OutOfRange));
        }
    }

    mod undo {
        use super::*;

        #[test]
        fn shuld_undo_last_operation() -> Result<()> {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_line = " added txt";
            table.add(new_line, initial_txt.len())?;
            assert_eq!(table.pieces.len(), 2);
            assert!(table.undo.is_empty());

            // when
            table.undo();

            // then
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(table.undo.len(), 1);

            Ok(())
        }
    }

    mod redo {
        use super::*;

        #[test]
        fn shuld_redo_last_operation() -> Result<()> {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_line = " added txt";
            table.add(new_line, initial_txt.len())?;
            table.undo();
            assert_eq!(table.pieces.len(), 1);
            assert_eq!(table.undo.len(), 1);

            // when
            table.redo();

            // then
            assert_eq!(table.pieces.len(), 2);
            assert!(table.undo.is_empty());

            Ok(())
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
        fn should_show_added_line_when_table_is_empty() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::default();
            let new_line = "some line";
            let cursor = 0;
            table.add(new_line, cursor)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, new_line);

            Ok(())
        }

        #[test]
        fn should_show_added_chars() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("a");
            let cursor = 1;
            table.add("b", cursor)?;
            table.add("c", cursor)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "abc");

            Ok(())
        }

        #[test]
        fn should_show_line_appended_at_the_end() -> Result<()> {
            init_logger();
            // given
            let initial_txt = "initial text";
            let mut table = PieceTable::from_text(initial_txt);
            let new_line = " some line";
            let cursor = initial_txt.len();
            table.add(new_line, cursor)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, format!("{initial_txt}{new_line}"));

            Ok(())
        }

        #[test]
        fn should_show_line_inserted_in_the_middle() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("some initial text");
            let new_line = "some line ";
            let cursor = 5;
            table.add(new_line, cursor)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(&txt, "some some line initial text");

            Ok(())
        }

        #[test]
        fn should_remove_char_from_the_middle() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove_char(7)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initialtext");

            Ok(())
        }

        #[test]
        fn should_remove_end_char_multiple_times() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove_char(11)?;
            table.remove_char(10)?;
            table.remove_char(9)?;
            table.remove_char(8)?;
            table.remove_char(7)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial");

            Ok(())
        }

        #[test]
        fn should_delete_consecutive_chars() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove_char(7)?;
            table.remove_char(8)?;
            table.remove_char(9)?;
            table.remove_char(10)?;
            table.remove_char(11)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial");

            Ok(())
        }

        #[test]
        fn should_remove_chars_at_the_end() -> Result<()> {
            init_logger();
            // given
            let initial_text = "initial text";
            let mut table = PieceTable::from_text(initial_text);
            table.remove_char(initial_text.len() - 1)?;
            table.remove_char(initial_text.len() - 2)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial te");

            Ok(())
        }

        #[test]
        fn should_not_show_removed_range() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text");
            table.remove(7..12)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial");

            Ok(())
        }
    }

    mod len {
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
            let piece_table = PieceTable::from_text(&initial_txt);

            // when
            let len = piece_table.len();

            // then
            assert_eq!(len, initial_txt.len());
        }

        #[test]
        fn len_takes_into_account_modifiec_piece_table() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("a");
            let cursor = 1;
            table.add("b", cursor)?;
            table.add("c", cursor)?;

            // when
            let len = table.len();

            // then
            assert_eq!(len, 3);

            Ok(())
        }
    }
}
