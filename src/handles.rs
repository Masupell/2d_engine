// Typed, generational handles.
//
// A handle = slot index + generation. Every slot stores its current generation.
// Removing bumps the slot's generation and frees the slot for reuse, so old handles
// to it no longer match: get() returns None instead of silently handing out
// whatever got loaded into that slot later.
//
// Each resource gets its own handle type, so the compiler stops you from passing
// a SoundId where a TextureId belongs.

use std::marker::PhantomData;

pub(crate) trait SlotKey: Copy
{
    fn from_parts(index: u32, generation: u32) -> Self;
    fn index(self) -> u32;
    fn generation(self) -> u32;
}

macro_rules! define_handle
{
    ($(#[$meta:meta])* $name:ident) =>
    {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct $name
        {
            index: u32,
            generation: u32
        }

        impl SlotKey for $name
        {
            fn from_parts(index: u32, generation: u32) -> Self { Self { index, generation } }
            fn index(self) -> u32 { self.index }
            fn generation(self) -> u32 { self.generation }
        }
    };
}

define_handle!(TextureId);
define_handle!(ShaderId);
define_handle!(SoundId); // Loaded Sound
define_handle!(PlaybackId); // Playing Sound

impl TextureId
{
    pub const WHITE: Self = Self { index: 0, generation: 0 };
}

impl ShaderId
{
    pub const DEFAULT: Self = Self { index: 0, generation: 0 };
}

impl PlaybackId
{
    pub const NONE: Self = Self { index: u32::MAX, generation: 0 };
}

struct Slot<T>
{
    generation: u32,
    value: Option<T>
}

pub(crate) struct Slots<K: SlotKey, T>
{
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    len: usize,
    _key: PhantomData<K>
}

impl<K: SlotKey, T> Slots<K, T>
{
    pub(crate) fn new() -> Self
    {
        Self { slots: Vec::new(), free: Vec::new(), len: 0, _key: PhantomData }
    }

    pub(crate) fn insert(&mut self, value: T) -> K
    {
        self.len += 1;

        if let Some(index) = self.free.pop()
        {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(value);
            return K::from_parts(index, slot.generation);
        }

        let index = self.slots.len() as u32;
        self.slots.push(Slot { generation: 0, value: Some(value) });
        K::from_parts(index, 0)
    }

    pub(crate) fn get(&self, key: K) -> Option<&T>
    {
        let slot = self.slots.get(key.index() as usize)?;
        if slot.generation != key.generation() { return None; }
        slot.value.as_ref()
    }

    pub(crate) fn get_mut(&mut self, key: K) -> Option<&mut T>
    {
        let slot = self.slots.get_mut(key.index() as usize)?;
        if slot.generation != key.generation() { return None; }
        slot.value.as_mut()
    }

    pub(crate) fn contains(&self, key: K) -> bool
    {
        self.get(key).is_some()
    }

    pub(crate) fn remove(&mut self, key: K) -> Option<T>
    {
        let index = key.index();
        let slot = self.slots.get_mut(index as usize)?;
        if slot.generation != key.generation() { return None; }

        let value = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1); // old handles stop matching right now
        self.free.push(index);
        self.len -= 1;
        Some(value)
    }

    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&T) -> bool)
    {
        for (index, slot) in self.slots.iter_mut().enumerate()
        {
            if slot.value.as_ref().is_some_and(|value| !keep(value))
            {
                slot.value = None;
                slot.generation = slot.generation.wrapping_add(1);
                self.free.push(index as u32);
                self.len -= 1;
            }
        }
    }

    /// Number of live values
    pub(crate) fn len(&self) -> usize
    {
        self.len
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &T>
    {
        self.slots.iter().filter_map(|slot| slot.value.as_ref())
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut T>
    {
        self.slots.iter_mut().filter_map(|slot| slot.value.as_mut())
    }

    // pub(crate) fn iter(&self) -> impl Iterator<Item = (K, &T)>
    // {
    //     self.slots.iter().enumerate().filter_map(|(index, slot)|
    //     {
    //         slot.value.as_ref().map(|value| (K::from_parts(index as u32, slot.generation), value))
    //     })
    // }
}

impl<K: SlotKey, T> Default for Slots<K, T>
{
    fn default() -> Self { Self::new() }
}
