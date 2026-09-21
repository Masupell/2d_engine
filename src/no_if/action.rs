#[derive(Copy, Clone, PartialEq)]
pub enum Action
{
    ToggleFullScreen,
    // ToggleVSync,
    // MoveLeft,
    // MoveRight,
    Escape,
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

    ToggleRopeExtending,
    // ToggleAnchorFalling

    StartGame,
    OpenSettings,
    BackToMainMenu,
    ResumeGame,
    RestartGame
}
impl Action { pub const COUNT: usize = 17; }
