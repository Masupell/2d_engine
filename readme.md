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

Other things as well for now:
- Asset Manager (so I can load assets at any time in the game, not just the start)
    * hashing, if texture is already loaded just return it (no need to have the same texture loaded twice)
    * Later dynamic loading? 
    * Async file loading (split loading of bigger assets into different threads)
    * Hot-reloading (detect changes during runtime, for assets, shaders, etc)
    * Unloading unused assets
    -> Support: Textures, Shaders, font-files, later sounds as well (I think thats everything important for now)
- Instead of using fixed indices for Textures, shaders, etc -> A handle system (handles the number for you , just maps to the actual number, but makes it easier to understand)
- Runtime Configuration Options, like:
    * Window Size (can do that already, by just dragging the screen)
    * Fullscreen toggle
    * Vsync toggle
    -> Display Settings
    * Target Framerate maybe?
    * background pausing enabled or not (paused when unfocused)
    * enable/disable post-processing (if not needed, will increase performance, because no need to render the view multiple times, just directly to screen)
    -> Performance
    * Hide Cursor + Custom Cursor
    -> Input
    -> Audio stuff for later (once I add audio)
    * Debug Tools (Wireframe mode, fps, etc)
- Maybe a Scene Handler? (kinda like in bevy, just do world.add_object(Object) and it renders all objects in world, not necessarily ecs though)
- Pass engine context, instead of renderer, input, etc (so, once things get added like setting or asset manager, there wont be more and more things in the function)
- Better Api, so I can just call draw() instead of engine.renderer.draw()
    * Chatgpt gave me this, just keep it in mind (so I have an idea for later):
        use std::cell::RefCell;

        thread_local! {
            static ENGINE_CONTEXT: RefCell<Option<*mut EngineContext<'static>>> = RefCell::new(None);
        }
        let mut ctx = EngineContext { ... };

        ENGINE_CONTEXT.with(|global| {
            *global.borrow_mut() = Some(&mut ctx as *mut _);
        });

        // now user code can call global draw_*, etc.
        app.update(&mut ctx, dt);
        app.render(&mut ctx);

        // after frame:
        ENGINE_CONTEXT.with(|global| *global.borrow_mut() = None);
        pub fn draw_texture(tex: Handle<Texture>, pos: (f32, f32), size: (f32, f32), rotation: f32) {
            ENGINE_CONTEXT.with(|global| {
                let ctx = unsafe { &mut *global.borrow().unwrap() };
                ctx.renderer.draw_texture(tex, pos, size, rotation);
            });
        }

        pub fn window_size() -> (f32, f32) {
            ENGINE_CONTEXT.with(|global| {
                let ctx = unsafe { &*global.borrow().unwrap() };
                ctx.renderer.window_size
            })
        }
- Should be all I need for now

1. EngineContext + better Api
2. AssetManager + Handle System
3. Runtime Config
4. Scene Handler (If I want to)
5. Debug tools (Once I need them)
6. Hot reloading (Not very important, makes it a lot faster to work with, but also can cause problems if not done correctly)