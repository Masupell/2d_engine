So far just a simple 2D Engine. Can draw Text, images and color-rects.
"cargo run -p engine" to run engine

Need to implement Text drawing, not just char, for that It would be usefull to implement an AtlasTexture (batchin in general)
ALso SDF-rendering, not bitmap-texture, for smooth-scaling

So steps:
1. Render text as one instead of singular chars
2. Make Texture drawing easier by creating a Texture struct for external use (not internal Texture)
3. Add extra Shapes, like circles, etc
4. Add Texture-batching

A basic start for a 2d-Game Engine.
Supports drawing quads of all sizes with rotation, color or a texture, also simple text.
Supports Pipeline switching, so you can have multiple shaders.

Things still to implement:

-Better text rendering:
(Old plan: Need to implement Text drawing, not just char, for that It would be usefull to implement an AtlasTexture (batchin in general)
ALso SDF-rendering, not bitmap-texture, for smooth-scaling, not necessaryhere, because htis is just for games, not animation

So steps:
1. Render text as one instead of singular chars
2. Make Texture drawing easier by creating a Texture struct for external use (not internal Texture)
3. Add extra Shapes, like circles, etc
4. Add Texture-batching)

- Post processing (Draw everything to a texture not directly to the screen, and then draw the texture, with any kind of shader)
- Texture-batching
- Texture Atlases
- Other shapes
- FOr diffeent shaders, make it easier to write new ones. (like bevy for example, manually add shades together, by concatenating the wgsl files, with like a shared wgsl file, so I dont have to include stuff the entire time again)