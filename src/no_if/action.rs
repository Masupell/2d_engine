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

    //Player
    PlaceCheckPoint,
    StartFalling,
    MoveUp,
    MoveLeft,
    MoveRight,
    Dash,

    ToggleRopeExtending,
    // ToggleAnchorFalling

    StartGame,
    OpenSettings,
    BackToMainMenu,
    ResumeGame,
    RestartGame,
    QuitGame,

    SelectWallShaderCracks,
    SelectWallShaderBands,
    SelectWallShaderFast
}
impl Action { pub const COUNT: usize = 21; }
