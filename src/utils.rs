use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn make_start_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::default().append_row([InlineKeyboardButton::callback(
        "РЖД",
            "rzd"
    )])
}

pub fn make_rzd_start_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::default().append_row([InlineKeyboardButton::callback(
        "Поиск купе",
        "rzd_search"
    )]).append_row(
        [InlineKeyboardButton::callback(
            "Задачи",
            "rzd_tasks"
        )]
    ).append_row(
        [InlineKeyboardButton::callback(
            "Назад",
            "rzd_return"
        )]
    )
}