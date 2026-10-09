A basic start for a 2d-Game Engine.
Supports drawing quads of all sizes with rotation, color or a texture, also simple text.
Supports Pipeline switching, so you can have multiple shaders.

Things still to implement:

- Text using atlas wiht default mesh instead of building it's own mesh; later using signed distance fields for texts
- ui depends currently on the world-camera -> second camera buffer for ui
- Culling (only pushing to draw commands if it is actually visible)
- Better draw Api (stuct based?)
- Handles for Texture, mesh and shader, instead of raw usize/u8
- camera zoom+rotation
- screen_to_world and world_to_screen functions
- Different Blend modes for different pipelines
- Different debug draw things (lines, circles, polygons)
- Threading system (engine handles threads, that game can use)
- Async loading of assets
- Hot reloading of shaders
- Gamepad support
- Frame stats for optimization
- Rect clipping (render_pass.set_scissor_rect)
