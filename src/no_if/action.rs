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
}
impl Action { pub const COUNT: usize = 10; }
