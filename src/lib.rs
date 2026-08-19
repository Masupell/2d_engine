pub mod state;
pub mod texture;
pub mod event_loop;
pub mod utility;
pub mod renderer;
pub mod input;
pub mod shader;
pub mod text;
pub mod context;
pub mod asset_manager;

pub mod no_if;

pub use event_loop::{EngineEvent, game_loop};
pub use context::{Context, UpdateContext, RenderContext, Loader};
pub use renderer::Renderer;
pub use input::Input;

pub use winit::keyboard::KeyCode as Key; // Temporary here
pub use no_if::button::*;
pub use no_if::vector::Vec2;
pub use no_if::action::Action;
pub use no_if::shapes::{Rect, Triangle};
pub use texture::FilterMode;
