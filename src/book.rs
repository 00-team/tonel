use std::fmt::Display;

use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::state::KeyData;

pub trait BookItem: Display {
    fn id(&self) -> i64;
}

pub struct Book<T: BookItem> {
    items: Vec<(i64, T)>,
    page: u32,
    max_page: u32,
}

impl<T: BookItem> Book<T> {
    pub fn new(items: Vec<T>, page: u32, max_page: u32) -> Self {
        let mut lm =
            Self { items: Vec::with_capacity(items.len()), page, max_page };
        for item in items {
            lm.items.push((item.id(), item));
        }

        lm
    }

    pub fn message(&self) -> String {
        let mut out = String::with_capacity(2048);
        for (id, item) in self.items.iter() {
            out += &format!("{id}. {item}\n");
        }
        out
    }

    pub fn keyboard(&self) -> InlineKeyboardMarkup {
        let mut layout = Vec::with_capacity(9);
        let next = (self.page + 1).min(self.max_page);
        let past = self.page.saturating_sub(1);
        layout.push(vec![
            InlineKeyboardButton::callback(
                self.max_page.to_string(),
                KeyData::BookPagination(self.max_page),
            ),
            InlineKeyboardButton::callback(
                next.to_string(),
                KeyData::BookPagination(next),
            ),
            InlineKeyboardButton::callback("add", KeyData::BookAdd),
            InlineKeyboardButton::callback(
                past.to_string(),
                KeyData::BookPagination(past),
            ),
            InlineKeyboardButton::callback("0", KeyData::BookPagination(0)),
        ]);

        let mut row = Vec::with_capacity(4);
        for (id, _) in self.items.iter() {
            if row.len() == 4 {
                layout.push(row);
                row = Vec::with_capacity(4);
            }
            row.push(InlineKeyboardButton::callback(
                id.to_string(),
                KeyData::BookItem(self.page, *id),
            ));
        }
        if !row.is_empty() {
            layout.push(row);
        }

        InlineKeyboardMarkup::new(layout)
    }
}
