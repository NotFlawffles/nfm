use crossterm::event::KeyCode;

pub enum Action {
    Close,
    Redraw,
    MoveUp,
    MoveDown,
    ScrollUp,
    ScrollDown,
    Home,
    End,
    ToggleHidden,
    Rename,
    Remove,
    Add,
    Open,
    Back,
    Search,
    ToggleHelp,
    Input(KeyCode),
}
