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

    //Player
    PlaceCheckPoint,
    StartFalling,
    MoveUp,
    MoveLeft,
    MoveRight,

    // Test
    Print,
    Hover,
    UnHover,
    Released
}
impl Action { pub const COUNT: usize = 14; }
