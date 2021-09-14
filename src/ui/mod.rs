use tui::{backend::Backend, layout::Rect, Frame};

use crate::app::App;

pub enum TableType {
    Album,
    Artist,
    Podcast,
    Playlist,
    Song,
}

#[derive(PartialEq)]
pub enum ColumnType {
    None,
    Title,
}

impl Default for ColumnType {
    fn default() -> Self {
        ColumnType::None
    }
}

pub struct TableHeader {
    id: TableType,
    items: Vec<TableHeaderItem>,
}

impl TableHeader {
    pub fn get_index(&self, id: ColumnType) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }
}

pub struct TableHeaderItem {
    id: ColumnType,
    text: String,
    width: u16,
}

pub struct TableItem {
    id: String,
    format: Vec<String>,
}

pub fn draw_main_layout<B: Backend>(f: &mut Frame<B>, app: &App, layout_chunk: Rect) {
    unimplemented!()
}

pub fn draw_routes<B: Backend>(f: &mut Frame<B>, app: &App, layout_chunk: Rect) {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
