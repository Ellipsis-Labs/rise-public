//! Minimal read-only sokoban ordered-list views for orderbook deserialization.
//!
//! These layouts mirror the read-only sokoban support used by
//! `phoenix-eternal-types`, but the constructors are fallible so short,
//! misaligned, or internally inconsistent account data returns
//! [`PhoenixAccountDecodeError`] instead of panicking.

use core::marker::PhantomData;

use bytemuck::{Pod, Zeroable};

use crate::common::PhoenixAccountDecodeError;

/// Sentinel value indicating null/end of list.
pub const SENTINEL: u32 = 0;

const NEXT_REG: usize = 1;

/// A node in a sokoban arena allocator with a fixed number of registers.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> {
    registers: [u32; NUM_REGISTERS],
    value: T,
}

unsafe impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> Zeroable
    for Node<T, NUM_REGISTERS>
{
}

unsafe impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> Pod
    for Node<T, NUM_REGISTERS>
{
}

impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize> Default
    for Node<T, NUM_REGISTERS>
{
    fn default() -> Self {
        Self {
            registers: [SENTINEL; NUM_REGISTERS],
            value: T::default(),
        }
    }
}

impl<T: Copy + Clone + Pod + Zeroable + Default, const NUM_REGISTERS: usize>
    Node<T, NUM_REGISTERS>
{
    #[inline(always)]
    pub const fn get_register(&self, r: usize) -> u32 {
        self.registers[r]
    }

    #[inline(always)]
    pub const fn get_value(&self) -> &T {
        &self.value
    }
}

pub type OrderedListNode<T> = Node<T, 2>;

/// A key-value node stored in a sokoban map.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct KVNode<K, V>
where
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
{
    pub key: K,
    pub value: V,
}

unsafe impl<K, V> Zeroable for KVNode<K, V>
where
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
{
}

unsafe impl<K, V> Pod for KVNode<K, V>
where
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
{
}

/// A fixed-size arena allocator for ordered-list maps.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SimpleNodeAllocator<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> {
    pub size: u64,
    bump_index: u32,
    free_list_head: u32,
    pub nodes: [OrderedListNode<T>; MAX_SIZE],
}

unsafe impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Zeroable
    for SimpleNodeAllocator<T, MAX_SIZE>
{
}

unsafe impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize> Pod
    for SimpleNodeAllocator<T, MAX_SIZE>
{
}

impl<T: Default + Copy + Clone + Pod + Zeroable, const MAX_SIZE: usize>
    SimpleNodeAllocator<T, MAX_SIZE>
{
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size as usize
    }

    #[inline(always)]
    pub fn get(&self, i: u32) -> &OrderedListNode<T> {
        &self.nodes[(i - 1) as usize]
    }

    #[inline(always)]
    pub fn try_get(&self, i: u32) -> Option<&OrderedListNode<T>> {
        if i == SENTINEL || i as usize > MAX_SIZE {
            None
        } else {
            Some(self.get(i))
        }
    }
}

/// Header for a static ordered-list map.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct OrderedListMapHeader<K, V>
where
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
{
    pub head: u32,
    pub tail: u32,
    _padding: [u32; 2],
    _phantom: PhantomData<(K, V)>,
}

unsafe impl<K, V> Zeroable for OrderedListMapHeader<K, V>
where
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
{
}

unsafe impl<K, V> Pod for OrderedListMapHeader<K, V>
where
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
{
}

/// Plain-old-data representation of a static ordered-list map.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StaticOrderedListMapPod<
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub header: OrderedListMapHeader<K, V>,
    pub allocator: SimpleNodeAllocator<KVNode<K, V>, MAX_SIZE>,
}

unsafe impl<
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> Zeroable for StaticOrderedListMapPod<K, V, MAX_SIZE>
{
}

unsafe impl<
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> Pod for StaticOrderedListMapPod<K, V, MAX_SIZE>
{
}

/// Read-only view into a static ordered-list map.
pub struct StaticOrderedListMap<
    'a,
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    inner: &'a StaticOrderedListMapPod<K, V, MAX_SIZE>,
}

impl<
    'a,
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> StaticOrderedListMap<'a, K, V, MAX_SIZE>
{
    pub fn try_load_from_buffer(
        account: &'static str,
        data: &'a [u8],
    ) -> Result<Self, PhoenixAccountDecodeError> {
        let len = core::mem::size_of::<StaticOrderedListMapPod<K, V, MAX_SIZE>>();
        let bytes = data
            .get(..len)
            .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
                account,
                expected: len,
                actual: data.len(),
            })?;
        let inner = bytemuck::try_from_bytes::<StaticOrderedListMapPod<K, V, MAX_SIZE>>(bytes)
            .map_err(|_| PhoenixAccountDecodeError::InvalidLayout { account })?;
        let map = Self { inner };
        map.validate(account)?;
        Ok(map)
    }

    pub fn validate(&self, account: &'static str) -> Result<(), PhoenixAccountDecodeError> {
        if self.len() > MAX_SIZE {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account,
                reason: "sokoban list length exceeds capacity",
            });
        }
        if self.is_empty() {
            if self.inner.header.head != SENTINEL || self.inner.header.tail != SENTINEL {
                return Err(PhoenixAccountDecodeError::InvalidData {
                    account,
                    reason: "empty sokoban list has non-null head or tail",
                });
            }
            return Ok(());
        }
        if self.inner.header.head == SENTINEL || self.inner.header.tail == SENTINEL {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account,
                reason: "non-empty sokoban list has null head or tail",
            });
        }

        let mut current = self.inner.header.head;
        for index in 0..self.len() {
            let Some(node) = self.inner.allocator.try_get(current) else {
                return Err(PhoenixAccountDecodeError::InvalidData {
                    account,
                    reason: "sokoban list node index is out of bounds",
                });
            };
            if index + 1 == self.len() && current != self.inner.header.tail {
                return Err(PhoenixAccountDecodeError::InvalidData {
                    account,
                    reason: "sokoban list tail does not match traversal",
                });
            }
            current = node.get_register(NEXT_REG);
        }
        if current != SENTINEL {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account,
                reason: "sokoban list traversal exceeds length",
            });
        }
        Ok(())
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.inner.allocator.size()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn get_head_index(&self) -> u32 {
        self.inner.header.head
    }

    #[inline(always)]
    pub fn get_next(&self, index: u32) -> u32 {
        self.inner
            .allocator
            .try_get(index)
            .map(|node| node.get_register(NEXT_REG))
            .unwrap_or(SENTINEL)
    }

    #[inline(always)]
    pub fn get_by_index(&self, index: u32) -> Option<(&K, &V)> {
        self.inner.allocator.try_get(index).map(|node| {
            let node_data = node.get_value();
            (&node_data.key, &node_data.value)
        })
    }

    pub fn iter(&self) -> StaticOrderedListMapIterator<'_, K, V, MAX_SIZE> {
        StaticOrderedListMapIterator {
            map: self,
            current: self.inner.header.head,
            remaining: self.len(),
        }
    }
}

/// Iterator over a static ordered-list map.
pub struct StaticOrderedListMapIterator<
    'a,
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    map: &'a StaticOrderedListMap<'a, K, V, MAX_SIZE>,
    current: u32,
    remaining: usize,
}

impl<
    'a,
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> Iterator for StaticOrderedListMapIterator<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 || self.current == SENTINEL {
            return None;
        }
        let node_data = self.map.inner.allocator.try_get(self.current)?.get_value();
        let result = (&node_data.key, &node_data.value);
        self.current = self.map.get_next(self.current);
        self.remaining -= 1;
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<
    K: PartialOrd + Ord + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> ExactSizeIterator for StaticOrderedListMapIterator<'_, K, V, MAX_SIZE>
{
}
