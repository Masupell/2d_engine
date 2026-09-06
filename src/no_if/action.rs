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

    ToggleWallShader,

    //Player
    PlaceCheckPoint,
    StartFalling,
    MoveUp,
    MoveLeft,
    MoveRight,

    ToggleRopeExtending
    // ToggleAnchorFalling
}
impl Action { pub const COUNT: usize = 12; }
