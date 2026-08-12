#[derive(Copy, Clone, PartialEq)]
pub enum Action
{
    ToggleFullScreen,
    // ToggleVSync,
    // MoveLeft,
    // MoveRight,
    Esc,
    // Mouse
    MouseLeftPressed,
    MouseLeftReleased,
    MouseLeftHold,
    Print, // Test
    Hover,
    UnHover,
    Released
}
impl Action { pub const COUNT: usize = 8; }
