//! Validated, immutable multi-account tree storage.

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::mem::size_of;

use bytemuck::Pod;

use crate::common::{PhoenixAccountDecodeError, read_pod, require_len, verify_discriminant};
use crate::discriminants::PhoenixAccount;
use crate::multi_arena::MultiArenaHeader;

const HEADER_LEN: usize = 96;
const ARENA_HEADER_LEN: usize = 32;
const REGISTERS_LEN: usize = 16;

#[derive(Debug)]
pub(crate) struct MultiArenaTree<'a, K> {
    header: MultiArenaHeader,
    root: u32,
    arenas: Vec<&'a [u8]>,
    capacity: usize,
    node_len: usize,
    _key: PhantomData<K>,
}

struct Node<'a, K> {
    left: u32,
    right: u32,
    parent: u32,
    color: u32,
    key: K,
    value: &'a [u8],
}

impl<'a, K: Pod + Ord> MultiArenaTree<'a, K> {
    pub(crate) fn try_from_account_buffers(
        buffers: impl IntoIterator<Item = &'a [u8]>,
        account_type: PhoenixAccount,
        arena_type: PhoenixAccount,
        value_len: usize,
    ) -> Result<Self, PhoenixAccountDecodeError> {
        let account = account_type.account_name();
        let invalid = |reason| PhoenixAccountDecodeError::InvalidData { account, reason };
        let mut buffers = buffers.into_iter();
        let first = buffers
            .next()
            .ok_or_else(|| invalid("missing header account"))?;
        require_len(account, first, HEADER_LEN)?;
        let header =
            MultiArenaHeader::try_from_account_bytes(account, first, account_type.discriminant())?;
        let block = header.superblock();
        if block.num_arenas() == 0 || block.num_nodes_per_arena() == 0 {
            return Err(invalid("uninitialized multi-arena tree"));
        }
        if block.num_active_arenas() > block.num_arenas() {
            return Err(invalid("active arena count exceeds registered arena count"));
        }
        let node_len = REGISTERS_LEN + size_of::<K>() + value_len;
        let mut arenas = vec![&first[HEADER_LEN..]];
        for (index, bytes) in buffers.enumerate() {
            if index + 1 >= usize::from(block.num_arenas()) {
                return Err(invalid("too many arena accounts"));
            }
            require_len(arena_type.account_name(), bytes, ARENA_HEADER_LEN)?;
            verify_discriminant(arena_type.account_name(), bytes, arena_type.discriminant())?;
            let actual_index = u16::from_le_bytes([bytes[8], bytes[9]]);
            if usize::from(actual_index) != index + 1 {
                return Err(invalid(
                    "arena accounts are missing, duplicated, or out of order",
                ));
            }
            arenas.push(&bytes[ARENA_HEADER_LEN..]);
        }
        if arenas.len() != usize::from(block.num_arenas()) {
            return Err(invalid("arena account count does not match the superblock"));
        }
        let mut capacity = 0usize;
        let mut saw_partial_arena = false;
        for bytes in &arenas {
            if bytes.len() % node_len != 0 {
                return Err(invalid("arena byte length is not a whole number of nodes"));
            }
            let nodes = bytes.len() / node_len;
            if nodes > block.num_nodes_per_arena() as usize || (saw_partial_arena && nodes != 0) {
                return Err(invalid("only the last populated arena may be partial"));
            }
            saw_partial_arena |= nodes < block.num_nodes_per_arena() as usize;
            capacity = capacity
                .checked_add(nodes)
                .ok_or_else(|| invalid("arena capacity overflow"))?;
        }
        if block.size() as usize > capacity
            || block.bump_index() == 0
            || block.bump_index() as u64 > capacity as u64 + 1
            || block.size() >= block.bump_index()
            || block.free_list_head() > block.bump_index()
        {
            return Err(invalid(
                "allocator length or node boundary exceeds capacity",
            ));
        }
        let root = u32::from_le_bytes([first[80], first[81], first[82], first[83]]);
        let tree = Self {
            header,
            root,
            arenas,
            capacity,
            node_len,
            _key: PhantomData,
        };
        tree.validate(account)?;
        Ok(tree)
    }

    fn validate(&self, account: &'static str) -> Result<(), PhoenixAccountDecodeError> {
        let invalid = |reason| PhoenixAccountDecodeError::InvalidData { account, reason };
        let mut pending = Vec::new();
        if self.root != 0 {
            pending.push((self.root, 0, None, None));
        }
        let mut count = 0;
        while let Some((index, parent, lower, upper)) = pending.pop() {
            count += 1;
            if count > self.len() {
                return Err(invalid("tree traversal exceeds declared length"));
            }
            let node = self
                .node(index)
                .ok_or_else(|| invalid("tree pointer is out of bounds"))?;
            if node.parent != parent || node.color > 1 {
                return Err(invalid("tree node has an invalid parent or color"));
            }
            if lower.is_some_and(|key| node.key <= key) || upper.is_some_and(|key| node.key >= key)
            {
                return Err(invalid("tree keys are not strictly ordered"));
            }
            if node.right != 0 {
                pending.push((node.right, index, Some(node.key), upper));
            }
            if node.left != 0 {
                pending.push((node.left, index, lower, Some(node.key)));
            }
        }
        if count != self.len() {
            return Err(invalid("tree traversal does not match declared length"));
        }
        Ok(())
    }

    fn node(&self, index: u32) -> Option<Node<'a, K>> {
        if index == 0 || index >= self.header.superblock().bump_index() {
            return None;
        }
        let page = index - 1;
        let per_arena = self.header.superblock().num_nodes_per_arena();
        let arena = self.arenas.get((page / per_arena) as usize)?;
        let offset = (page % per_arena) as usize * self.node_len;
        let bytes = arena.get(offset..offset + self.node_len)?;
        let word = |offset| {
            u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ])
        };
        Some(Node {
            left: word(0),
            right: word(4),
            parent: word(8),
            color: word(12),
            key: bytemuck::try_pod_read_unaligned(
                &bytes[REGISTERS_LEN..REGISTERS_LEN + size_of::<K>()],
            )
            .ok()?,
            value: &bytes[REGISTERS_LEN + size_of::<K>()..],
        })
    }

    pub(crate) fn header(&self) -> &MultiArenaHeader {
        &self.header
    }

    pub(crate) fn len(&self) -> usize {
        self.header.superblock().size() as usize
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    pub(crate) fn get(&self, key: &K) -> Option<(u32, &'a [u8])> {
        let mut index = self.root;
        while index != 0 {
            let node = self.node(index)?;
            match key.cmp(&node.key) {
                Ordering::Less => index = node.left,
                Ordering::Greater => index = node.right,
                Ordering::Equal => return Some((index, node.value)),
            }
        }
        None
    }

    pub(crate) fn get_by_index(&self, index: u32) -> Option<(K, &'a [u8])> {
        let node = self.node(index)?;
        let (live_index, bytes) = self.get(&node.key)?;
        (live_index == index).then_some((node.key, bytes))
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (u32, K, &'a [u8])> + '_ {
        let mut stack = Vec::new();
        let mut current = self.root;
        std::iter::from_fn(move || {
            while current != 0 {
                stack.push(current);
                current = self.node(current)?.left;
            }
            let index = stack.pop()?;
            let node = self.node(index)?;
            current = node.right;
            Some((index, node.key, node.value))
        })
    }

    pub(crate) fn read_value<V: Pod>(&self, bytes: &[u8]) -> Option<V> {
        read_pod("multi-arena entry", bytes).ok()
    }
}
