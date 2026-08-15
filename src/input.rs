use std::collections::{HashMap, HashSet};

use winit::{event::{ElementState, KeyEvent, MouseButton, WindowEvent}, keyboard::{KeyCode, PhysicalKey}};
use crate::no_if::action::Action;

pub struct Input
{
    keys_pressed: HashSet<KeyCode>,
    prev_keys_pressed: HashSet<KeyCode>,
    mouse_pressed: HashSet<MouseButton>,
    prev_mouse_pressed: HashSet<MouseButton>,
    mouse_position: Option<(f64, f64)>,
    window_size: (f64, f64),
    virtual_size: (f64, f64),
    actions: Vec<Action>,
    key_bindings: HashMap<KeyCode, KeyBinding>,
    mouse_bindings: HashMap<MouseButton, MouseBinding>
}

impl Input
{
    pub(crate) fn new(window_size: (f64, f64)) -> Self
    {
        let mut key_bindings = HashMap::new();
        key_bindings.insert(KeyCode::F11, KeyBinding { pressed: Some(Action::ToggleFullScreen), released: None, hold: None});
        // key_bindings.insert(KeyCode::KeyQ, Action::ToggleVSync);
        // key_bindings.insert(KeyCode::KeyA, Action::MoveLeft);
        // key_bindings.insert(KeyCode::KeyD, Action::MoveRight);
        // key_bindings.insert(KeyCode::Escape, Action::Esc);

        let mut mouse_bindings = HashMap::new();
        mouse_bindings.insert(MouseButton::Left, MouseBinding { pressed: Some(Action::MouseLeftPressed), released: Some(Action::MouseLeftReleased), hold: Some(Action::MouseLeftHold)});


        Self
        {
            keys_pressed: HashSet::new(),
            prev_keys_pressed: HashSet::new(),
            mouse_pressed: HashSet::new(),
            prev_mouse_pressed: HashSet::new(),
            mouse_position: None,
            window_size,
            virtual_size: window_size,
            actions: Vec::new(),
            key_bindings,
            mouse_bindings
        }
    }

    pub(crate) fn update_inputs(&mut self, event: &WindowEvent)
    {
        if let WindowEvent::KeyboardInput
            {
                event: KeyEvent
                {
                    state,
                    physical_key: PhysicalKey::Code(key),
                    ..
                },
                ..
            } = event
        {
            match state
            {
                ElementState::Pressed => { self.keys_pressed.insert(*key); }
                ElementState::Released => { self.keys_pressed.remove(key); }
            }
        }

        if let WindowEvent::MouseInput
            {
                state,
                button,
                ..
            } = event
        {
            match state
            {
                ElementState::Pressed => { self.mouse_pressed.insert(*button); }
                ElementState::Released => { self.mouse_pressed.remove(button); }
            }
        }

        if let WindowEvent::CursorMoved { position, ..} = event
        {
            self.mouse_position = Some((position.x, position.y));
        }
    }

    pub(crate) fn prev_update(&mut self)
    {
        self.generate_actions();
        self.prev_keys_pressed = self.keys_pressed.clone();
        self.prev_mouse_pressed = self.mouse_pressed.clone();
    }

    pub(crate) fn update_screen(&mut self, size: (f64, f64))
    {
        self.window_size = size;
    }

    pub fn actual_mouse_position(&self) -> (f64, f64)
    {
        if let Some(mouse_pos) = self.mouse_position
        {
            return mouse_pos;
        }
        return (0.0, 0.0);
    }

    pub fn mouse_position(&self) -> (f64, f64)
    {
        if let Some(mouse_pos) = self.mouse_position
        {
            return (mouse_pos.0/self.window_size.0*self.virtual_size.0, mouse_pos.1/self.window_size.1*self.virtual_size.1);
        }
        return (0.0, 0.0);
    }

    pub fn mouse_position_f32(&self) -> (f32, f32)
    {
        if let Some(mouse_pos) = self.mouse_position
        {
            return ((mouse_pos.0/self.window_size.0*self.virtual_size.0) as f32, (mouse_pos.1/self.window_size.1*self.virtual_size.1) as f32);
        }
        return (0.0, 0.0);
    }

    pub fn generate_actions(&mut self)
    {
        self.actions.clear();

        for key in &self.keys_pressed
        {
            if let Some(binding) = self.key_bindings.get(key)
            {
                if !self.prev_keys_pressed.contains(key)
                {
                    if let Some(action) = binding.pressed
                    {
                        self.actions.push(action);
                    }
                }

                if let Some(action) = binding.hold
                {
                    self.actions.push(action);
                }
            }
        }

        for key in &self.prev_keys_pressed
        {
            if !self.keys_pressed.contains(key)
            {
                if let Some(binding) = self.key_bindings.get(key)
                {
                    if let Some(action) = binding.released
                    {
                        self.actions.push(action);
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
                        self.actions.push(action);
                    }
                }

                if let Some(action) = binding.hold
                {
                    self.actions.push(action);
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
                        self.actions.push(action);
                    }
                }
            }
        }
    }

    pub fn add_key_binding(&mut self, key: KeyCode, pressed: Option<Action>, released: Option<Action>, held: Option<Action>)
    {
        self.key_bindings.insert(key, KeyBinding { pressed, released, hold: held });
    }

    pub(crate) fn add_action(&mut self, action: Action)
    {
        self.actions.push(action);
    }

    pub fn actions(&self) -> &[Action]
    {
        &self.actions
    }
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
