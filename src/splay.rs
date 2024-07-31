use crate::common::{Direction, Node, NodeRef};
use std::array::from_fn;
use std::cmp::PartialOrd;
use std::fmt::Debug;

pub trait NodeArena<T: Clone + Copy + Debug + Eq + PartialEq>: Debug {
    fn node(&self, internal_id: T) -> &Node<T>;
    fn node_mut(&mut self, internal_id: T) -> &mut Node<T>;
    fn root_idx(&self) -> NodeRef<T>;
    fn root_idx_mut(&mut self) -> &mut T;
    fn ref_internal(&self, internal_id: T) -> NodeRef<T>;

    fn is_consistent(&self) -> bool;
    // TODO: 'incr' is an ugly wart, but sadly there's just no good way to express the concept "u8 or u16".
    fn incr(&self, v: T) -> T;

    fn is_subtree_consistent(&self, root_index: T, cover_min: T, cover_max_incl: T) -> bool
    where
        T: PartialOrd,
    {
        let node = &self.node(root_index);
        // eprintln!("ENTER internal node {root_index}={node:?} cover_min={cover_min}, cover_max_incl={cover_max_incl}");
        let index_consistent = cover_min <= root_index && root_index < cover_max_incl;
        if !index_consistent {
            eprintln!(
                "internal node {root_index:?} not consistent: cover_min={cover_min:?}, cover_max_incl={cover_max_incl:?}"
            );
        }
        let left_consistent = self.is_arm_consistent(&node.left, cover_min, root_index);
        let right_consistent =
            self.is_arm_consistent(&node.right, self.incr(root_index), cover_max_incl);
        if !left_consistent || !right_consistent {
            eprintln!(
                "internal node {root_index:?} has inconsistent arms: cover_min={cover_min:?}, cover_max_incl={cover_max_incl:?}"
            );
        }
        //eprintln!("EXIT internal node {root_index} cover_min={cover_min}, cover_max_incl={cover_max_incl}");
        index_consistent && left_consistent && right_consistent
    }

    fn is_arm_consistent(&self, root: &NodeRef<T>, cover_min: T, cover_max_incl: T) -> bool
    where
        T: PartialOrd,
    {
        if let Some(child_index) = root.as_internal() {
            return self.is_subtree_consistent(child_index, cover_min, cover_max_incl);
        }
        if let Some(leaf_index) = root.as_leaf() {
            return cover_min == leaf_index && leaf_index == cover_max_incl;
        }
        panic!("empty child?!")
    }

    fn splayable_mut(&mut self) -> Splayable<'_, T, Self> {
        Splayable::new(self)
    }
}

#[derive(Debug)]
pub struct Arena8 {
    internal_nodes: [Node<u8>; u8::MAX as usize],
    // A leaf is always "right before" its corresponding internal node, if any.
    // That must be this way around, because there is a leaf 255 but no internal node 255. (Or 65535.)
    root: u8,
}

impl Arena8 {
    pub fn new_uniform() -> Self {
        let nodes: [Node<u8>; u8::MAX as usize] = from_fn(|i| {
            let level = i.trailing_ones();
            assert!(level < u8::BITS);
            let ibu = i as u8;
            if level == 0 {
                Node {
                    left: NodeRef::new_leaf(ibu),
                    right: NodeRef::new_leaf(ibu + 1),
                }
            } else {
                let masked = ibu & !(1 << (level - 1));
                let added_bit = 1 << level;
                Node {
                    left: NodeRef::new_internal(masked, u8::MAX),
                    right: NodeRef::new_internal(masked | added_bit, u8::MAX),
                }
            }
        });
        Self {
            internal_nodes: nodes,
            root: u8::MAX / 2,
        }
    }
}

impl NodeArena<u8> for Arena8 {
    fn node(&self, internal_id: u8) -> &Node<u8> {
        &self.internal_nodes[internal_id as usize]
    }

    fn node_mut(&mut self, internal_id: u8) -> &mut Node<u8> {
        &mut self.internal_nodes[internal_id as usize]
    }

    fn root_idx(&self) -> NodeRef<u8> {
        NodeRef::new_internal(self.root, u8::MAX)
    }

    fn root_idx_mut(&mut self) -> &mut u8 {
        &mut self.root
    }

    fn ref_internal(&self, internal_id: u8) -> NodeRef<u8> {
        NodeRef::new_internal(internal_id, u8::MAX)
    }

    fn incr(&self, v: u8) -> u8 {
        v + 1
    }

    fn is_consistent(&self) -> bool {
        self.is_subtree_consistent(self.root, 0, u8::MAX)
    }
}

#[derive(Debug)]
pub struct Splayable<'a, T: Clone + Copy + Debug + Eq + PartialEq, A: NodeArena<T> + ?Sized> {
    arena: &'a mut A,
    node: NodeRef<T>,
    internal_parents: Vec<(T, Direction)>,
}

impl<'a, T: Clone + Copy + Debug + Eq + PartialEq, A: NodeArena<T> + ?Sized> Splayable<'a, T, A> {
    fn new(arena: &'a mut A) -> Self {
        let node = arena.root_idx();
        Self {
            arena,
            node,
            internal_parents: Vec::with_capacity(std::mem::size_of::<T>() * 2),
        }
    }

    pub fn current_value(&self) -> T {
        match self.node {
            NodeRef::Internal(v) => v,
            NodeRef::Leaf(v) => v,
        }
    }

    pub fn is_root(&self) -> bool {
        self.internal_parents.is_empty()
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self.node, NodeRef::Leaf(_))
    }

    pub fn go(&mut self, dir: Direction) {
        let node_id = match self.node {
            NodeRef::Internal(v) => v,
            _ => panic!("Tried to descend on leaf?!"),
        };
        self.internal_parents.push((node_id, dir));
        let node = &self.arena.node(node_id);
        self.node = node.arm(dir);
    }

    pub fn find_deep_internal(&self, min_length: usize) -> T {
        assert!(self.is_root());
        assert!(!self.is_leaf());
        let mut level = 0;
        let mut candidates = vec![self.node.as_internal().unwrap()];
        while level < min_length {
            level += 1;
            assert!(!candidates.is_empty());
            let mut next_candidates = Vec::with_capacity(candidates.len() * 2);
            for candidate_id in &candidates {
                let node = &self.arena.node(*candidate_id);
                for d in [Direction::Left, Direction::Right] {
                    let noderef = node.arm(d);
                    if let Some(child_id) = noderef.as_internal() {
                        next_candidates.push(child_id);
                    }
                }
            }
            candidates = next_candidates;
        }
        assert!(!candidates.is_empty());
        candidates[0]
    }

    pub fn is_consistent(&self) -> bool {
        self.arena.is_consistent()
    }

    pub fn splay_parent_of_leaf(&mut self) {
        assert!(self.is_leaf());
        self.node = self
            .arena
            .ref_internal(self.internal_parents.pop().unwrap().0);
        self.splay_internal();
    }

    fn splay_internal(&mut self) {
        assert!(!self.is_leaf());
        let node_id = self.node.as_internal().expect("Suddenly leaf!?");
        while self.internal_parents.len() >= 2 {
            let (parent_id, parent_dir) = self
                .internal_parents
                .pop()
                .expect("length should be >= 2?!");
            assert_eq!(self.arena.node(parent_id).arm(parent_dir), self.node);
            let (grandparent_id, grandparent_dir) = self
                .internal_parents
                .pop()
                .expect("length should be >= 2?!");
            assert_eq!(
                self.arena.node(grandparent_id).arm(grandparent_dir),
                self.arena.ref_internal(parent_id)
            );

            // We're about to replace grandparent by node, so first update the pointer to grandparent:
            if let Some(&(ggp_id, ggp_dir)) = self.internal_parents.last() {
                assert_eq!(
                    self.arena.node(ggp_id).arm(ggp_dir),
                    self.arena.ref_internal(grandparent_id)
                );
                // -1 ref to grandparent, +1 ref to self.node
                *self.arena.node_mut(ggp_id).arm_mut(ggp_dir) = self.node;
            } else {
                // -1 ref to grandparent, +1 ref to self.node
                *self.arena.root_idx_mut() = node_id;
            }

            if grandparent_dir == parent_dir {
                // println!("Doing zigzig gp_dir={grandparent_dir:?} p_dir={parent_dir:?}");
                // zigzig (A\B\C becomes A/B/C)
                // Before:
                //           G
                //     a           P
                //              b     N
                //                   c d
                // After:
                //           N
                //     P           d
                //  G     c
                // a b
                let subtree_b = self.arena.node(parent_id).arm(parent_dir.opposite());
                let subtree_c = self.arena.node(node_id).arm(parent_dir.opposite());
                // -1 ref to 'subtree_b', +1 ref to grandparent
                *self
                    .arena
                    .node_mut(parent_id)
                    .arm_mut(parent_dir.opposite()) = self.arena.ref_internal(grandparent_id);
                // -1 ref to parent, +1 ref to 'subtree_b'
                *self.arena.node_mut(grandparent_id).arm_mut(grandparent_dir) = subtree_b;
                // -1 ref to 'subtree_c', +1 ref to parent
                *self.arena.node_mut(node_id).arm_mut(parent_dir.opposite()) =
                    self.arena.ref_internal(parent_id);
                // -1 ref to self.node +1 ref to 'subtree_c'
                *self.arena.node_mut(parent_id).arm_mut(parent_dir) = subtree_c;
                // Should be consistent again.
            } else {
                // println!("Doing zigzag gp_dir={grandparent_dir:?} p_dir={parent_dir:?}");
                assert_eq!(grandparent_dir, parent_dir.opposite());
                // zigzag (">" becomes "nAn")
                // Before:
                //           G
                //     a           P
                //              N     d
                //             b c
                // After:
                //           N
                //     G           P
                //  a     b     c     d
                let subtree_b = self.arena.node(node_id).arm(parent_dir);
                let subtree_c = self.arena.node(node_id).arm(grandparent_dir);
                // -1 ref to 'subtree_b', +1 ref to grandparent
                *self.arena.node_mut(node_id).arm_mut(parent_dir) =
                    self.arena.ref_internal(grandparent_id);
                // -1 ref to parent, +1 ref to 'subtree_b'
                *self.arena.node_mut(grandparent_id).arm_mut(grandparent_dir) = subtree_b;
                // -1 ref to 'subtree_c', +1 ref to parent
                *self.arena.node_mut(node_id).arm_mut(grandparent_dir) =
                    self.arena.ref_internal(parent_id);
                // -1 ref to self.node +1 ref to 'subtree_c'
                *self.arena.node_mut(parent_id).arm_mut(parent_dir) = subtree_c;
                // Should be consistent again.
            }
        }
        if !self.internal_parents.is_empty() {
            // zig (only near root)
            // Before:
            //      P
            //   N     c
            //  a b
            // After:
            //     N
            //  a     P
            //       b c
            let (parent_id, parent_dir) = self
                .internal_parents
                .pop()
                .expect("length should be == 1?!");
            // println!("Doing zig p_dir={parent_dir:?}");
            assert!(self.internal_parents.is_empty());
            assert_eq!(self.arena.node(parent_id).arm(parent_dir), self.node);
            assert_eq!(Some(parent_id), self.arena.root_idx().as_internal());

            // We're about to replace root == parent, so first update that pointer:
            // -1 ref to parent, +1 ref to self.node
            *self.arena.root_idx_mut() = node_id;

            let subtree_b = self.arena.node(node_id).arm(parent_dir.opposite());
            // -1 ref to 'subtree_b', +1 ref to parent
            *self.arena.node_mut(node_id).arm_mut(parent_dir.opposite()) =
                self.arena.ref_internal(parent_id);
            // -1 ref to self.node, +1 ref to 'subtree_b'
            *self.arena.node_mut(parent_id).arm_mut(parent_dir) = subtree_b;
            // Should be consistent again.
        }
        assert!(self.internal_parents.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_is_consistent() {
        let tree = Arena8::new_uniform();
        // eprintln!("{tree:?}");
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_tree_structure() {
        let tree = Arena8::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(tree.internal_nodes[0].left, NodeRef::new_leaf(0));
        assert_eq!(tree.internal_nodes[0].right, NodeRef::new_leaf(1));
        assert_eq!(
            tree.internal_nodes[1].left,
            NodeRef::new_internal(0, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[1].right,
            NodeRef::new_internal(2, u8::MAX)
        );
        assert_eq!(tree.internal_nodes[2].left, NodeRef::new_leaf(2));
        assert_eq!(tree.internal_nodes[2].right, NodeRef::new_leaf(3));
        assert_eq!(
            tree.internal_nodes[3].left,
            NodeRef::new_internal(1, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[3].right,
            NodeRef::new_internal(5, u8::MAX)
        );
        assert_eq!(tree.internal_nodes[4].left, NodeRef::new_leaf(4));
        assert_eq!(tree.internal_nodes[4].right, NodeRef::new_leaf(5));
        assert_eq!(
            tree.internal_nodes[5].left,
            NodeRef::new_internal(4, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[5].right,
            NodeRef::new_internal(6, u8::MAX)
        );
        assert_eq!(tree.internal_nodes[6].left, NodeRef::new_leaf(6));
        assert_eq!(tree.internal_nodes[6].right, NodeRef::new_leaf(7));
    }

    #[test]
    fn test_go_basic() {
        let mut tree = Arena8::new_uniform();
        let mut walker = tree.splayable_mut(); // [0, 255]
        assert_eq!(127, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        assert_eq!(127, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Right); // [128, 255]
        assert_eq!(128 + 63, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Left); // [128, 191]
        assert_eq!(128 + 31, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Right); // [160, 191]
        assert_eq!(160 + 15, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Left); // [160, 175]
        assert_eq!(160 + 7, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Right); // [168, 175]
        assert_eq!(168 + 3, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Left); // [168, 171]
        assert_eq!(168 + 1, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Right); // [170, 171]
        assert_eq!(170 + 0, walker.current_value());
        assert_eq!(false, walker.is_leaf());
        walker.go(Direction::Left); // [170, 170]
        assert_eq!(170, walker.current_value());
        assert_eq!(true, walker.is_leaf());
    }

    #[test]
    fn test_splay_noop() {
        let mut tree = Arena8::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(
            tree.internal_nodes[127].left,
            NodeRef::new_internal(63, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63, u8::MAX)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.splay_internal();
        }
        assert_eq!(tree.root, 127);
        assert_eq!(
            tree.internal_nodes[127].left,
            NodeRef::new_internal(63, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63, u8::MAX)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zig_left() {
        let mut tree = Arena8::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(
            tree.internal_nodes[127].left,
            NodeRef::new_internal(63, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[63].left,
            NodeRef::new_internal(31, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[63].right,
            NodeRef::new_internal(64 + 31, u8::MAX)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Left);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 63);
        assert_eq!(
            tree.internal_nodes[63].left,
            NodeRef::new_internal(31, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[63].right,
            NodeRef::new_internal(127, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].left,
            NodeRef::new_internal(64 + 31, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63, u8::MAX)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zig_right() {
        let mut tree = Arena8::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(
            tree.internal_nodes[127].left,
            NodeRef::new_internal(63, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[128 + 63].left,
            NodeRef::new_internal(128 + 31, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[128 + 63].right,
            NodeRef::new_internal(128 + 64 + 31, u8::MAX)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 128 + 63);
        assert_eq!(
            tree.internal_nodes[128 + 63].left,
            NodeRef::new_internal(127, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[128 + 63].right,
            NodeRef::new_internal(128 + 64 + 31, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].left,
            NodeRef::new_internal(63, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 31, u8::MAX)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzig_left() {
        let mut tree = Arena8::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].left,
            NodeRef::new_internal(0x1f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].right,
            NodeRef::new_internal(0x5f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x1f].left,
            NodeRef::new_internal(0x0f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x1f].right,
            NodeRef::new_internal(0x2f, u8::MAX)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Left);
            walker.go(Direction::Left);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0x1f);
        assert_eq!(
            tree.internal_nodes[0x1f].left,
            NodeRef::new_internal(0x0f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x1f].right,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].left,
            NodeRef::new_internal(0x2f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].right,
            NodeRef::new_internal(0x7f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x5f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzig_right() {
        let mut tree = Arena8::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].left,
            NodeRef::new_internal(0x9f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].right,
            NodeRef::new_internal(0xdf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xdf].left,
            NodeRef::new_internal(0xcf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xdf].right,
            NodeRef::new_internal(0xef, u8::MAX)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0xdf);
        assert_eq!(
            tree.internal_nodes[0xdf].left,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xdf].right,
            NodeRef::new_internal(0xef, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].left,
            NodeRef::new_internal(0x7f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].right,
            NodeRef::new_internal(0xcf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0x9f, u8::MAX)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzag_rightleft() {
        let mut tree = Arena8::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].left,
            NodeRef::new_internal(0x9f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].right,
            NodeRef::new_internal(0xdf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x9f].left,
            NodeRef::new_internal(0x8f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x9f].right,
            NodeRef::new_internal(0xaf, u8::MAX)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.go(Direction::Left);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0x9f);
        assert_eq!(
            tree.internal_nodes[0x9f].left,
            NodeRef::new_internal(0x7f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x9f].right,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0x8f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].left,
            NodeRef::new_internal(0xaf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0xbf].right,
            NodeRef::new_internal(0xdf, u8::MAX)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzag_leftright() {
        let mut tree = Arena8::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].left,
            NodeRef::new_internal(0x1f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].right,
            NodeRef::new_internal(0x5f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x5f].left,
            NodeRef::new_internal(0x4f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x5f].right,
            NodeRef::new_internal(0x6f, u8::MAX)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Left);
            walker.go(Direction::Right);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0x5f);
        assert_eq!(
            tree.internal_nodes[0x5f].left,
            NodeRef::new_internal(0x3f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x5f].right,
            NodeRef::new_internal(0x7f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].left,
            NodeRef::new_internal(0x1f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x3f].right,
            NodeRef::new_internal(0x4f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].left,
            NodeRef::new_internal(0x6f, u8::MAX)
        );
        assert_eq!(
            tree.internal_nodes[0x7f].right,
            NodeRef::new_internal(0xbf, u8::MAX)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzig_zigzag_zig() {
        let mut tree = Arena8::new_uniform();
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Left);
            walker.go(Direction::Left);
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0x3b);
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzag_zigzig() {
        let mut tree = Arena8::new_uniform();
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.go(Direction::Left);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0xe7);
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_leaf() {
        let mut tree = Arena8::new_uniform();
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.go(Direction::Left);
            walker.go(Direction::Left);
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.go(Direction::Left);
            walker.go(Direction::Left);
            walker.splay_parent_of_leaf();
        }
        assert_eq!(tree.root, 0b1001_1100);
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_dir_roundtrip() {
        assert_eq!(
            Direction::Right,
            Direction::from_bit(Direction::Right.to_bit())
        );
        assert_eq!(
            Direction::Left,
            Direction::from_bit(Direction::Left.to_bit())
        );
        assert!(Direction::from_bit(true).to_bit());
        assert!(!Direction::from_bit(false).to_bit());
    }
}
