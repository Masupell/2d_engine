use std::collections::{HashMap, HashSet};

use winit::{event::{ElementState, MouseButton, WindowEvent}, keyboard::{Key, KeyCode, NamedKey, PhysicalKey}};
use crate::no_if::action::Action;

pub struct Input
{
    keys_pressed: HashSet<KeyCode>,
    prev_keys_pressed: HashSet<KeyCode>,
    mouse_pressed: HashSet<MouseButton>,
    prev_mouse_pressed: HashSet<MouseButton>,
    mouse_position: (f64, f64),
    window_size: (f64, f64),
    view_size: (f64, f64),
    virtual_size: (f64, f64),
    viewport: (f64, f64, f64, f64),
    once_actions: Vec<Action>,
    hold_actions: Vec<Action>,
    actions: Vec<Action>, // only for the gamee
    key_bindings: HashMap<KeyCode, KeyBinding>,
    mouse_bindings: HashMap<MouseButton, MouseBinding>,
    any_press: bool,

    pending_typed: String,
    pending_backspaces: u32,
    pending_enter: bool,
    typed: String,
    backspaces: u32, // amount of presses
    enter_pressed: bool,
    capture_requested: bool, // a text field wants the keyboard input
    capturing_text: bool // disables keybindings while true
}

impl Input
{
    pub(crate) fn new(window_size: (f64, f64)) -> Self
    {
        let mut key_bindings = HashMap::new();
        key_bindings.insert(KeyCode::F11, KeyBinding { pressed: Some(Action::ToggleFullScreen), released: None, hold: None});

        let mut mouse_bindings = HashMap::new();
        mouse_bindings.insert(MouseButton::Left, MouseBinding { pressed: Some(Action::MouseLeftPressed), released: Some(Action::MouseLeftReleased), hold: Some(Action::MouseLeftHold)});


        Self
        {
            keys_pressed: HashSet::new(),
            prev_keys_pressed: HashSet::new(),
            mouse_pressed: HashSet::new(),
            prev_mouse_pressed: HashSet::new(),
            mouse_position: (window_size.0/2.0, window_size.1/2.0),
            window_size,
            view_size: window_size,
            virtual_size: window_size,
            viewport: (0.0, 0.0, window_size.0, window_size.1),
            once_actions: Vec::new(),
            hold_actions: Vec::new(),
            actions: Vec::new(),
            key_bindings,
            mouse_bindings,
            any_press: false,
            pending_typed: String::new(),
            pending_backspaces: 0,
            pending_enter: false,
            typed: String::new(),
            backspaces: 0,
            enter_pressed: false,
            capture_requested: false,
            capturing_text: false
        }
    }

    pub(crate) fn update_inputs(&mut self, event: &WindowEvent)
    {
        if let WindowEvent::KeyboardInput { event: key_event, ..} = event
        {
            if let PhysicalKey::Code(key) = key_event.physical_key
            {
                match key_event.state
                {
                    ElementState::Pressed => { self.keys_pressed.insert(key); }
                    ElementState::Released => { self.keys_pressed.remove(&key); }
                }
            }

            if key_event.state == ElementState::Pressed
            {
                if let Some(text) = &key_event.text
                {
                    self.pending_typed.extend(text.chars().filter(|c| !c.is_control()));
                }

                match key_event.logical_key
                {
                    Key::Named(NamedKey::Backspace) => self.pending_backspaces += 1,
                    Key::Named(NamedKey::Enter) => self.pending_enter = true,
                    _ => {}
                }
            }
        }

        if let WindowEvent::MouseInput {state, button, ..} = event
        {
            match state
            {
                ElementState::Pressed => { self.mouse_pressed.insert(*button); }
                ElementState::Released => { self.mouse_pressed.remove(button); }
            }
        }

        if let WindowEvent::CursorMoved { position, ..} = event
        {
            self.mouse_position = (position.x, position.y);
        }
    }

    pub(crate) fn prev_update(&mut self)
    {
        self.typed.push_str(&std::mem::take(&mut self.pending_typed));
        self.backspaces += std::mem::take(&mut self.pending_backspaces);
        self.enter_pressed |= std::mem::take(&mut self.pending_enter);

        self.generate_actions();
        self.prev_keys_pressed = self.keys_pressed.clone();
        self.prev_mouse_pressed = self.mouse_pressed.clone();
    }

    pub(crate) fn consume_once(&mut self)
    {
        self.once_actions.clear();
        self.any_press = false;
        self.actions.clear();
        self.actions.extend_from_slice(&self.hold_actions);

        self.typed.clear();
        self.backspaces = 0;
        self.enter_pressed = false;

        self.capturing_text = self.capture_requested;
        self.capture_requested = false;
    }

    pub(crate) fn update_screen(&mut self, size: (f64, f64), viewport: (f32, f32, f32, f32), view_size: (f32, f32))
    {
        self.window_size = size;
        self.viewport = (viewport.0 as f64, viewport.1 as f64, viewport.2 as f64, viewport.3 as f64);
        self.view_size = (view_size.0 as f64, view_size.1 as f64);
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

    pub fn generate_actions(&mut self)
    {
        self.hold_actions.clear();

        self.any_press |= self.keys_pressed.iter().any(|key| !self.prev_keys_pressed.contains(key)) || self.mouse_pressed.iter().any(|button| !self.prev_mouse_pressed.contains(button));

        let capturing = self.capturing_text;

        for key in self.keys_pressed.iter().filter(|_| !capturing)
        {
            if let Some(binding) = self.key_bindings.get(key)
            {
                if !self.prev_keys_pressed.contains(key)
                {
                    if let Some(action) = binding.pressed
                    {
                        self.once_actions.push(action);
                    }
                }

                if let Some(action) = binding.hold
                {
                    self.hold_actions.push(action);
                }
            }
        }

        for key in self.prev_keys_pressed.iter().filter(|_| !capturing)
        {
            if !self.keys_pressed.contains(key)
            {
                if let Some(binding) = self.key_bindings.get(key)
                {
                    if let Some(action) = binding.released
                    {
                        self.once_actions.push(action);
                    }
                }
            }
        }

        for button in &self.mouse_pressed
        {
            if let Some(binding) = self.mouse_bindings.get(button)
            {
                if !self.prev_mouse_pressed.contains(button)
                {
                    if let Some(action) = binding.pressed
                    {
                        self.once_actions.push(action);
                    }
                }

                if let Some(action) = binding.hold
                {
                    self.hold_actions.push(action);
                }
            }
        }

        for button in &self.prev_mouse_pressed
        {
            if !self.mouse_pressed.contains(button)
            {
                if let Some(binding) = self.mouse_bindings.get(button)
                {
                    if let Some(action) = binding.released
                    {
                        self.once_actions.push(action);
                    }
                }
            }
        }

        self.actions.clear();
        self.actions.extend_from_slice(&self.once_actions);
        self.actions.extend_from_slice(&self.hold_actions);
    }

    pub fn add_key_binding(&mut self, key: KeyCode, pressed: Option<Action>, released: Option<Action>, held: Option<Action>)
    {
        self.key_bindings.insert(key, KeyBinding { pressed, released, hold: held });
    }

    pub fn add_mouse_binding(&mut self, button: MouseButton, pressed: Option<Action>, released: Option<Action>, held: Option<Action>)
    {
        self.mouse_bindings.insert(button, MouseBinding { pressed, released, hold: held });
    }

    pub(crate) fn add_action(&mut self, action: Action)
    {
        self.actions.push(action);
    }

    pub fn actions(&self) -> &[Action]
    {
        &self.actions
    }

    pub fn any_free_press(&self, is_used: impl Fn(Action) -> bool) -> bool
    {
        let claimed = self.actions.iter().any(|&action| is_used(action));
        self.any_press && !claimed
    }


    pub fn typed_text(&self) -> &str { &self.typed }
    pub fn backspaces(&self) -> u32 { self.backspaces }
    pub fn enter_pressed(&self) -> bool { self.enter_pressed }

    pub fn request_text_capture(&mut self) { self.capture_requested = true; }
}

struct KeyBinding
{
    pressed: Option<Action>,
    released: Option<Action>,
    hold: Option<Action>
}

struct MouseBinding
{
    pressed: Option<Action>,
    released: Option<Action>,
    hold: Option<Action>
}
