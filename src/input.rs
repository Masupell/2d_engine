use std::collections::HashSet;

use winit::{event::{ElementState, KeyEvent, MouseButton, WindowEvent}, keyboard::{KeyCode, PhysicalKey}};

pub struct Input
{
    keys_pressed: HashSet<KeyCode>,
    prev_keys_pressed: HashSet<KeyCode>,
    mouse_pressed: HashSet<MouseButton>,
    prev_mouse_pressed: HashSet<MouseButton>,
    mouse_position: Option<(f64, f64)>,
    window_size: (f64, f64),
    virtual_size: (f64, f64)
}

impl Input
{
    pub(crate) fn new(window_size: (f64, f64)) -> Self
    {
        Self
        {
            keys_pressed: HashSet::new(),
            prev_keys_pressed: HashSet::new(),
            mouse_pressed: HashSet::new(),
            prev_mouse_pressed: HashSet::new(),
            mouse_position: None,
            window_size,
            virtual_size: window_size
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
        self.prev_keys_pressed = self.keys_pressed.clone();
        self.prev_mouse_pressed = self.mouse_pressed.clone();
    }

    pub(crate) fn update_screen(&mut self, size: (f64, f64))
    {
        self.window_size = size;
    }

    pub fn is_key_hold(&self, key: KeyCode) -> bool
    {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool
    {
        self.keys_pressed.contains(&key) && !self.prev_keys_pressed.contains(&key)
    }

    pub fn is_mouse_hold(&self, button: MouseButton) -> bool
    {
        self.mouse_pressed.contains(&button)
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool
    {
        self.mouse_pressed.contains(&button) && !self.prev_mouse_pressed.contains(&button)
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
}