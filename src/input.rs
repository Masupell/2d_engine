use std::collections::HashSet;

use winit::{event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent}, keyboard::{Key, KeyCode, NamedKey, PhysicalKey}};

#[derive(Default)]
struct FrameInput
{
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    scroll: (f32, f32)
}

impl FrameInput
{
    fn clear(&mut self)
    {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.scroll = (0.0, 0.0);
    }

    fn any_press(&self) -> bool
    {
        !self.keys_pressed.is_empty() || !self.mouse_pressed.is_empty()
    }
}

struct HitRegion
{
    id: u32,
    min: (f32, f32),
    max: (f32, f32),
    z: u32
}

pub struct Input
{
    keys_down: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,

    frame: FrameInput, // inputs for current frame
    physics: FrameInput,
    in_physics: bool,

    mouse_position: (f64, f64),
    window_size: (f64, f64),
    view_size: (f64, f64),
    virtual_size: (f64, f64),
    viewport: (f64, f64, f64, f64),

    typed: String,
    backspaces: u32, // amount of presses
    enter_pressed: bool,
    esc_pressed: bool,

    capture_requested: bool, // a text field wants the keyboard input
    capturing_text: bool, // key queries return false while true

    hit_regions: Vec<HitRegion>,
    hovered: Option<u32>, // each element that should block, gets its own ID
    focus: Option<u32> // for anything focused, useful in wrold (like when there is an inworld scroll-list)
}

impl Input
{
    pub(crate) fn new(window_size: (f64, f64)) -> Self
    {
        Self
        {
            keys_down: HashSet::new(),
            mouse_down: HashSet::new(),
            frame: FrameInput::default(),
            physics: FrameInput::default(),
            in_physics: false,
            mouse_position: (window_size.0 / 2.0, window_size.1 / 2.0),
            window_size,
            view_size: window_size,
            virtual_size: window_size,
            viewport: (0.0, 0.0, window_size.0, window_size.1),
            typed: String::new(),
            backspaces: 0,
            enter_pressed: false,
            esc_pressed: false,
            capture_requested: false,
            capturing_text: false,
            hit_regions: Vec::new(),
            hovered: None,
            focus: None
        }
    }

    pub(crate) fn update_inputs(&mut self, event: &WindowEvent)
    {
        match event
        {
            WindowEvent::KeyboardInput { event: key_event, .. } =>
            {
                if let PhysicalKey::Code(key) = key_event.physical_key
                {
                    match key_event.state
                    {
                        ElementState::Pressed =>
                        {
                            if self.keys_down.insert(key)
                            {
                                self.frame.keys_pressed.insert(key);
                                self.physics.keys_pressed.insert(key);
                            }
                        }
                        ElementState::Released =>
                        {
                            if self.keys_down.remove(&key)
                            {
                                self.frame.keys_released.insert(key);
                                self.physics.keys_released.insert(key);
                            }
                        }
                    }
                }

                if key_event.state == ElementState::Pressed
                {
                    if let Some(text) = &key_event.text
                    {
                        self.typed.extend(text.chars().filter(|c| !c.is_control()));
                    }

                    match key_event.logical_key
                    {
                        Key::Named(NamedKey::Backspace) => self.backspaces += 1,
                        Key::Named(NamedKey::Enter) => self.enter_pressed = true,
                        Key::Named(NamedKey::Escape) => self.esc_pressed = true,
                        _ => {}
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } =>
            {
                match state
                {
                    ElementState::Pressed =>
                    {
                        if self.mouse_down.insert(*button)
                        {
                            self.frame.mouse_pressed.insert(*button);
                            self.physics.mouse_pressed.insert(*button);
                        }
                    }
                    ElementState::Released =>
                    {
                        if self.mouse_down.remove(button)
                        {
                            self.frame.mouse_released.insert(*button);
                            self.physics.mouse_released.insert(*button);
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, ..} =>
            {
                const PIXELS_PER_LINE: f32 = 20.0;

                let (x, y) = match delta
                {
                    MouseScrollDelta::LineDelta(x, y) => (*x, *y),
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as f32 / PIXELS_PER_LINE, pos.y as f32 / PIXELS_PER_LINE)
                };

                self.frame.scroll.0 += x;
                self.frame.scroll.1 += y;
                self.physics.scroll.0 += x;
                self.physics.scroll.1 += y;
            }
            WindowEvent::CursorMoved { position, .. } =>
            {
                self.mouse_position = (position.x, position.y);
            }
            WindowEvent::Focused(false) =>
            {
                self.keys_down.clear();
                self.mouse_down.clear();
            }
            _ => {}
        }
    }

    pub(crate) fn begin_physics_step(&mut self)
    {
        self.in_physics = true;
    }

    pub(crate) fn end_physics_step(&mut self)
    {
        self.in_physics = false;
        self.physics.clear();
    }

    pub(crate) fn end_frame(&mut self)
    {
        let (mx, my) = self.mouse_position_f32();
        self.hovered = self.hit_regions.iter().filter(|r| mx >= r.min.0 && mx < r.max.0 && my >= r.min.1 && my < r.max.1).max_by_key(|r| r.z).map(|r| r.id);
        self.hit_regions.clear();

        self.frame.clear();

        self.typed.clear();
        self.backspaces = 0;
        self.enter_pressed = false;
        self.esc_pressed = false;

        self.capturing_text = self.capture_requested;
        self.capture_requested = false;
    }

    pub(crate) fn update_screen(&mut self, size: (f64, f64), viewport: (f32, f32, f32, f32), view_size: (f32, f32))
    {
        // Approximation, only works, when window was centered on screen
        let (old_width, old_height) = self.window_size;
        if old_width > 0.0 && old_height > 0.0
        {
            self.mouse_position.0 *= size.0 / old_width;
            self.mouse_position.1 *= size.1 / old_height;
        }

        self.window_size = size;
        self.viewport = (viewport.0 as f64, viewport.1 as f64, viewport.2 as f64, viewport.3 as f64);
        self.view_size = (view_size.0 as f64, view_size.1 as f64);
    }

    fn edges(&self) -> &FrameInput
    {
        if self.in_physics { &self.physics } else { &self.frame }
    }



    pub fn is_key_hold(&self, key: KeyCode) -> bool
    {
        !self.capturing_text && self.keys_down.contains(&key)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool
    {
        !self.capturing_text && self.edges().keys_pressed.contains(&key)
    }

    pub fn is_key_released(&self, key: KeyCode) -> bool
    {
        !self.capturing_text && self.edges().keys_released.contains(&key)
    }



    pub fn is_mouse_hold(&self, button: MouseButton) -> bool
    {
        self.mouse_down.contains(&button)
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool
    {
        self.edges().mouse_pressed.contains(&button)
    }

    pub fn is_mouse_released(&self, button: MouseButton) -> bool
    {
        self.edges().mouse_released.contains(&button)
    }

    pub fn any_press(&self) -> bool
    {
        self.edges().any_press()
    }




    pub fn actual_mouse_position(&self) -> (f64, f64)
    {
        self.mouse_position
    }

    pub fn actual_mouse_position_f32(&self) -> (f32, f32)
    {
        let (x, y) = self.mouse_position;
        (x as f32, y as f32)
    }

    // centered virtual_size inside of view_size, left and above goes negative, to the right and bottom higher than virtual_size
    pub fn mouse_position(&self) -> (f64, f64)
    {
        let (px, py) = self.mouse_position;
        let (vx, vy, vw, vh) = self.viewport;

        let x = (px - vx) * (self.view_size.0 / vw) - (self.view_size.0 - self.virtual_size.0) * 0.5;
        let y = (py - vy) * (self.view_size.1 / vh) - (self.view_size.1 - self.virtual_size.1) * 0.5;

        (x, y)
    }

    pub fn mouse_position_f32(&self) -> (f32, f32)
    {
        let (x, y) = self.mouse_position();
        (x as f32, y as f32)
    }

    // 0..1 across the whole window including bars
    pub fn mouse_position_normalized(&self) -> (f64, f64)
    {
        let (x, y) = self.mouse_position;
        (x / self.window_size.0, y / self.window_size.1)
    }

    pub fn mouse_position_normalized_f32(&self) -> (f32, f32)
    {
        let (x, y) = self.mouse_position_normalized();
        (x as f32, y as f32)
    }

    pub fn virtual_to_normalized(&self, x: f64, y: f64) -> (f64, f64)
    {
        let (vx, vy, vw, vh) = self.viewport;
        let px = (x + (self.view_size.0 - self.virtual_size.0) * 0.5) * (vw / self.view_size.0) + vx;
        let py = (y + (self.view_size.1 - self.virtual_size.1) * 0.5) * (vh / self.view_size.1) + vy;
        (px / self.window_size.0, py / self.window_size.1)
    }



    pub fn scroll(&self) -> (f32, f32)
    {
        self.edges().scroll
    }

    pub fn scroll_y(&self) -> f32
    {
        self.edges().scroll.1
    }

    pub fn scrolled_up(&self) -> bool
    {
        self.edges().scroll.1 > 0.0
    }

    pub fn scrolled_down(&self) -> bool
    {
        self.edges().scroll.1 < 0.0
    }



    // Register a UI region (tope-left+size), run every frame
    pub fn hover(&mut self, id: u32, pos: (f32, f32), size: (f32, f32), z: u32) -> bool
    {
        self.hit_regions.push(HitRegion { id, min: pos, max: (pos.0 + size.0, pos.1 + size.1), z });
        self.hovered == Some(id)
    }

    // If any registered ui is under the mouse
    pub fn pointer_over_ui(&self) -> bool
    {
        self.hovered.is_some()
    }

    pub fn hovered(&self) -> Option<u32> { self.hovered }

    pub fn set_focus(&mut self, id: u32) { self.focus = Some(id); }
    pub fn clear_focus(&mut self) { self.focus = None; }
    pub fn has_focus(&self, id: u32) -> bool { self.focus == Some(id) }
    pub fn focus(&self) -> Option<u32> { self.focus }


    pub fn typed_text(&self) -> &str { &self.typed }
    pub fn backspaces(&self) -> u32 { self.backspaces }
    pub fn enter_pressed(&self) -> bool { self.enter_pressed }
    pub fn esc_pressed(&self) -> bool { self.esc_pressed }

    pub fn request_text_capture(&mut self) { self.capture_requested = true; }
    pub fn is_capturing_text(&self) -> bool { self.capturing_text }
}
