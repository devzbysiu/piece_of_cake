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
}

impl<'a> PieceTable<'a> {
    pub fn from_text(txt: &'a str) -> Self {
        Self {
            original_buffer: txt,
            add_buffer: String::new(),
            pieces: vec![Piece::new(0, txt.len(), Source::Original)],
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
        self.add_buffer.push_str(txt)
    }

    fn original_buffer(&self) -> &str {
        &self.original_buffer
    }

    fn add_piece(&mut self, add_piece: Piece) {
        self.pieces.push(add_piece)
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
        self.pieces.insert(current_idx, first_piece)
    }

    fn append_from(&self, txt: &mut String, piece: &Piece) {
        let buff = match piece.source {
            Source::Original => &self.original_buffer[piece.start..piece.end],
            Source::Add => &self.add_buffer[piece.start..piece.end],
        };
        txt.push_str(buff);
    }

    pub fn remove_char(&mut self, cursor_idx: usize) -> Result<()> {
        let Some(current_piece_idx) = self.find_current_piece_idx(cursor_idx) else {
            error!("cursor outside of the text");
            return Err(ModificationError::OutOfRange);
        };
        trace!("current piece idx: {current_piece_idx:?}");
        let mut current_piece = self.remove_piece(current_piece_idx);
        if current_piece.start == cursor_idx {
            trace!("modifying start");
            trace!("before: {current_piece:#?}");
            current_piece.start = current_piece.start + 1;
            trace!("after: {current_piece:#?}");
            self.insert_piece(current_piece_idx, current_piece);
        } else if current_piece.end == cursor_idx {
            trace!("modifying end");
            trace!("before: {current_piece:#?}");
            current_piece.end = current_piece.end - 1;
            trace!("end: {current_piece:#?}");
            self.insert_piece(current_piece_idx, current_piece);
        } else {
            trace!("splitting at {cursor_idx}");
            trace!("before {current_piece:#?}");
            let (first_piece, mut second_piece) = current_piece.split_at(cursor_idx);
            trace!("after {first_piece:#?} & {second_piece:#?}");
            trace!("modifying in the middle");
            trace!("before: {second_piece:#?}");
            second_piece.start = second_piece.start + 1;
            trace!("after: {second_piece:#?}");
            self.insert_piece(current_piece_idx, first_piece);
            self.insert_piece(current_piece_idx + 1, second_piece);
        }
        Ok(())
    }

    pub fn remove(&mut self, range: Range<usize>) -> Result<()> {
        for cursor_idx in range.rev() {
            self.remove_char(cursor_idx)?;
        }
        Ok(())
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

    use env_logger;

    fn init_logger() {
        let _ = env_logger::try_init();
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
            let mut table = PieceTable::from_text(initial_txt.into());
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
            let mut table = PieceTable::from_text("some initial text".into());
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

        #[test]
        fn should_not_show_removed_char() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text".into());
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
            let mut table = PieceTable::from_text("initial text".into());
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
            let mut table = PieceTable::from_text("initial text".into());
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
        fn should_not_show_removed_range() -> Result<()> {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text".into());
            table.remove(7..12)?;

            // when
            let txt = table.project();

            // then
            assert_eq!(txt, "initial");

            Ok(())
        }

        #[test]
        fn should_return_out_of_range_error_when_removing_at_wrong_cursor() {
            init_logger();
            // given
            let mut table = PieceTable::from_text("initial text".into());

            // when
            let res = table.remove(7..150);

            // then
            assert_eq!(res, Err(ModificationError::OutOfRange));
        }
    }
}
