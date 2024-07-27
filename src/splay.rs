use std::array::from_fn;

type BaseUnit = u8;

// Causes stack overflow by allocating 262 KiB?!
// type BaseUnit = u16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeRef {
    Internal(BaseUnit),
    Leaf(BaseUnit),
}

impl NodeRef {
    fn new_leaf(v: BaseUnit) -> NodeRef {
        NodeRef::Leaf(v)
    }

    fn new_internal(v: BaseUnit) -> NodeRef {
        assert!(v != BaseUnit::MAX, "too large internal ID: {v}");
        NodeRef::Internal(v)
    }

    fn as_leaf(&self) -> Option<BaseUnit> {
        match self {
            NodeRef::Leaf(v) => Some(*v),
            _ => None,
        }
    }

    fn as_internal(&self) -> Option<BaseUnit> {
        match self {
            NodeRef::Internal(v) => Some(*v),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Node {
    left: NodeRef,
    right: NodeRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Left,
    Right,
}

impl Direction {
    fn opposite(&self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

impl Node {
    fn arm(&self, dir: Direction) -> NodeRef {
        match dir {
            Direction::Left => self.left,
            Direction::Right => self.right,
        }
    }

    fn arm_mut(&mut self, dir: Direction) -> &mut NodeRef {
        match dir {
            Direction::Left => &mut self.left,
            Direction::Right => &mut self.right,
        }
    }
}

#[derive(Debug)]
struct SplayTree {
    internal_nodes: [Node; BaseUnit::MAX as usize],
    // A leaf is always "right before" its corresponding internal node, if any.
    // That must be this way around, because there is a leaf 255 but no internal node 255. (Or 65535.)
    root: BaseUnit,
}

impl SplayTree {
    fn new_uniform() -> SplayTree {
        let nodes: [Node; BaseUnit::MAX as usize] = from_fn(|i| {
            let level = i.trailing_ones();
            assert!(level < BaseUnit::BITS);
            let ibu = i as BaseUnit;
            if level == 0 {
                Node {
                    left: NodeRef::new_leaf(ibu),
                    right: NodeRef::new_leaf(ibu + 1),
                }
            } else {
                let masked = ibu & !(1 << (level - 1));
                let added_bit = 1 << level;
                Node {
                    left: NodeRef::new_internal(masked),
                    right: NodeRef::new_internal(masked | added_bit),
                }
            }
        });
        SplayTree {
            internal_nodes: nodes,
            root: BaseUnit::MAX / 2,
        }
    }

    fn is_consistent(&self) -> bool {
        self.is_subtree_consistent(self.root, 0, BaseUnit::MAX)
    }

    fn is_subtree_consistent(
        &self,
        root_index: BaseUnit,
        cover_min: BaseUnit,
        cover_max_incl: BaseUnit,
    ) -> bool {
        let node = &self.internal_nodes[root_index as usize];
        // eprintln!("ENTER internal node {root_index}={node:?} cover_min={cover_min}, cover_max_incl={cover_max_incl}");
        let index_consistent = cover_min <= root_index && root_index < cover_max_incl;
        if !index_consistent {
            eprintln!("internal node {root_index} not consistent: cover_min={cover_min}, cover_max_incl={cover_max_incl}");
        }
        let left_consistent = self.is_arm_consistent(&node.left, cover_min, root_index);
        let right_consistent = self.is_arm_consistent(&node.right, root_index + 1, cover_max_incl);
        if !left_consistent || !right_consistent {
            eprintln!("internal node {root_index} has inconsistent arms: cover_min={cover_min}, cover_max_incl={cover_max_incl}");
        }
        //eprintln!("EXIT internal node {root_index} cover_min={cover_min}, cover_max_incl={cover_max_incl}");
        index_consistent && left_consistent && right_consistent
    }

    fn is_arm_consistent(
        &self,
        root: &NodeRef,
        cover_min: BaseUnit,
        cover_max_incl: BaseUnit,
    ) -> bool {
        if let Some(child_index) = root.as_internal() {
            return self.is_subtree_consistent(child_index, cover_min, cover_max_incl);
        }
        if let Some(leaf_index) = root.as_leaf() {
            return cover_min == leaf_index && leaf_index == cover_max_incl;
        }
        panic!("empty child?!")
    }

    fn splayable_mut(&mut self) -> Splayable {
        Splayable::new(self)
    }
}

#[derive(Debug)]
struct Splayable<'a> {
    tree: &'a mut SplayTree,
    node: NodeRef,
    // TODO: This should live in SplayTree, not here, for memory allocation purposes.
    internal_parents: Vec<(BaseUnit, Direction)>,
}

impl<'a> Splayable<'a> {
    fn new(tree: &'a mut SplayTree) -> Self {
        let node = NodeRef::new_internal(tree.root);
        Self {
            tree,
            node,
            internal_parents: Vec::with_capacity(BaseUnit::BITS as usize * 2),
        }
    }

    fn current_value(&self) -> BaseUnit {
        match self.node {
            NodeRef::Internal(v) => v,
            NodeRef::Leaf(v) => v,
        }
    }

    fn is_leaf(&self) -> bool {
        matches!(self.node, NodeRef::Leaf(_))
    }

    fn go(&mut self, dir: Direction) {
        let node_id = match self.node {
            NodeRef::Internal(v) => v,
            _ => panic!("Tried to descend on leaf?!"),
        };
        self.internal_parents.push((node_id, dir));
        let node = &self.tree.internal_nodes[node_id as usize];
        self.node = node.arm(dir);
    }

    fn splay_parent_of_leaf(&mut self) {
        assert!(self.is_leaf());
        self.node = NodeRef::new_internal(self.internal_parents.pop().unwrap().0);
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
            assert_eq!(
                self.tree.internal_nodes[parent_id as usize].arm(parent_dir),
                self.node
            );
            let (grandparent_id, grandparent_dir) = self
                .internal_parents
                .pop()
                .expect("length should be >= 2?!");
            assert_eq!(
                self.tree.internal_nodes[grandparent_id as usize].arm(grandparent_dir),
                NodeRef::new_internal(parent_id)
            );

            // We're about to replace grandparent by node, so first update the pointer to grandparent:
            if let Some(&(ggp_id, ggp_dir)) = self.internal_parents.last() {
                assert_eq!(
                    self.tree.internal_nodes[ggp_id as usize].arm(ggp_dir),
                    NodeRef::new_internal(grandparent_id)
                );
                // -1 ref to grandparent, +1 ref to self.node
                *self.tree.internal_nodes[ggp_id as usize].arm_mut(ggp_dir) = self.node;
            } else {
                // -1 ref to grandparent, +1 ref to self.node
                self.tree.root = node_id;
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
                let subtree_b =
                    self.tree.internal_nodes[parent_id as usize].arm(parent_dir.opposite());
                let subtree_c =
                    self.tree.internal_nodes[node_id as usize].arm(parent_dir.opposite());
                // -1 ref to 'subtree_b', +1 ref to grandparent
                *self.tree.internal_nodes[parent_id as usize].arm_mut(parent_dir.opposite()) =
                    NodeRef::new_internal(grandparent_id);
                // -1 ref to parent, +1 ref to 'subtree_b'
                *self.tree.internal_nodes[grandparent_id as usize].arm_mut(grandparent_dir) =
                    subtree_b;
                // -1 ref to 'subtree_c', +1 ref to parent
                *self.tree.internal_nodes[node_id as usize].arm_mut(parent_dir.opposite()) =
                    NodeRef::new_internal(parent_id);
                // -1 ref to self.node +1 ref to 'subtree_c'
                *self.tree.internal_nodes[parent_id as usize].arm_mut(parent_dir) = subtree_c;
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
                let subtree_b = self.tree.internal_nodes[node_id as usize].arm(parent_dir);
                let subtree_c = self.tree.internal_nodes[node_id as usize].arm(grandparent_dir);
                // -1 ref to 'subtree_b', +1 ref to grandparent
                *self.tree.internal_nodes[node_id as usize].arm_mut(parent_dir) =
                    NodeRef::new_internal(grandparent_id);
                // -1 ref to parent, +1 ref to 'subtree_b'
                *self.tree.internal_nodes[grandparent_id as usize].arm_mut(grandparent_dir) =
                    subtree_b;
                // -1 ref to 'subtree_c', +1 ref to parent
                *self.tree.internal_nodes[node_id as usize].arm_mut(grandparent_dir) =
                    NodeRef::new_internal(parent_id);
                // -1 ref to self.node +1 ref to 'subtree_c'
                *self.tree.internal_nodes[parent_id as usize].arm_mut(parent_dir) = subtree_c;
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
            assert_eq!(
                self.tree.internal_nodes[parent_id as usize].arm(parent_dir),
                self.node
            );
            assert_eq!(parent_id, self.tree.root);

            // We're about to replace root == parent, so first update that pointer:
            // -1 ref to parent, +1 ref to self.node
            self.tree.root = node_id;

            let subtree_b = self.tree.internal_nodes[node_id as usize].arm(parent_dir.opposite());
            // -1 ref to 'subtree_b', +1 ref to parent
            *self.tree.internal_nodes[node_id as usize].arm_mut(parent_dir.opposite()) =
                NodeRef::new_internal(parent_id);
            // -1 ref to self.node, +1 ref to 'subtree_b'
            *self.tree.internal_nodes[parent_id as usize].arm_mut(parent_dir) = subtree_b;
            // Should be consistent again.
        }
        assert!(self.internal_parents.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_sizes() {
        // "1 + padding" = size_of::<BaseUnit>(), meh
        assert_eq!(
            size_of::<BaseUnit>() + size_of::<BaseUnit>(),
            size_of::<NodeRef>()
        );
    }

    #[test]
    fn test_ref_leaf() {
        let leaf_0 = NodeRef::new_leaf(0);
        let leaf_10 = NodeRef::new_leaf(10);
        let leaf_max = NodeRef::new_leaf(BaseUnit::MAX);
        assert_eq!(leaf_0.as_leaf(), Some(0));
        assert_eq!(leaf_10.as_leaf(), Some(10));
        assert_eq!(leaf_max.as_leaf(), Some(BaseUnit::MAX));
        assert_eq!(leaf_0.as_internal(), None);
        assert_eq!(leaf_10.as_internal(), None);
        assert_eq!(leaf_max.as_internal(), None);
    }

    #[test]
    fn test_ref_internal() {
        let int_0 = NodeRef::new_internal(0);
        let int_10 = NodeRef::new_internal(10);
        let int_max = NodeRef::new_internal(BaseUnit::MAX - 1);
        assert_eq!(int_0.as_internal(), Some(0));
        assert_eq!(int_10.as_internal(), Some(10));
        assert_eq!(int_max.as_internal(), Some(BaseUnit::MAX - 1));
        assert_eq!(int_0.as_leaf(), None);
        assert_eq!(int_10.as_leaf(), None);
        assert_eq!(int_max.as_leaf(), None);
    }

    #[test]
    #[should_panic = "too large internal ID"]
    fn test_ref_internal_overflow() {
        NodeRef::new_internal(BaseUnit::MAX);
    }

    #[test]
    fn test_uniform_is_consistent() {
        let tree = SplayTree::new_uniform();
        // eprintln!("{tree:?}");
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_tree_structure() {
        let tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(tree.internal_nodes[0].left, NodeRef::new_leaf(0));
        assert_eq!(tree.internal_nodes[0].right, NodeRef::new_leaf(1));
        assert_eq!(tree.internal_nodes[1].left, NodeRef::new_internal(0));
        assert_eq!(tree.internal_nodes[1].right, NodeRef::new_internal(2));
        assert_eq!(tree.internal_nodes[2].left, NodeRef::new_leaf(2));
        assert_eq!(tree.internal_nodes[2].right, NodeRef::new_leaf(3));
        assert_eq!(tree.internal_nodes[3].left, NodeRef::new_internal(1));
        assert_eq!(tree.internal_nodes[3].right, NodeRef::new_internal(5));
        assert_eq!(tree.internal_nodes[4].left, NodeRef::new_leaf(4));
        assert_eq!(tree.internal_nodes[4].right, NodeRef::new_leaf(5));
        assert_eq!(tree.internal_nodes[5].left, NodeRef::new_internal(4));
        assert_eq!(tree.internal_nodes[5].right, NodeRef::new_internal(6));
        assert_eq!(tree.internal_nodes[6].left, NodeRef::new_leaf(6));
        assert_eq!(tree.internal_nodes[6].right, NodeRef::new_leaf(7));
    }

    #[test]
    fn test_go_basic() {
        let mut tree = SplayTree::new_uniform();
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
        let mut tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(tree.internal_nodes[127].left, NodeRef::new_internal(63));
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.splay_internal();
        }
        assert_eq!(tree.root, 127);
        assert_eq!(tree.internal_nodes[127].left, NodeRef::new_internal(63));
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zig_left() {
        let mut tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(tree.internal_nodes[127].left, NodeRef::new_internal(63));
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63)
        );
        assert_eq!(tree.internal_nodes[63].left, NodeRef::new_internal(31));
        assert_eq!(
            tree.internal_nodes[63].right,
            NodeRef::new_internal(64 + 31)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Left);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 63);
        assert_eq!(tree.internal_nodes[63].left, NodeRef::new_internal(31));
        assert_eq!(tree.internal_nodes[63].right, NodeRef::new_internal(127));
        assert_eq!(
            tree.internal_nodes[127].left,
            NodeRef::new_internal(64 + 31)
        );
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zig_right() {
        let mut tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 127);
        assert_eq!(tree.internal_nodes[127].left, NodeRef::new_internal(63));
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 63)
        );
        assert_eq!(
            tree.internal_nodes[128 + 63].left,
            NodeRef::new_internal(128 + 31)
        );
        assert_eq!(
            tree.internal_nodes[128 + 63].right,
            NodeRef::new_internal(128 + 64 + 31)
        );
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 128 + 63);
        assert_eq!(
            tree.internal_nodes[128 + 63].left,
            NodeRef::new_internal(127)
        );
        assert_eq!(
            tree.internal_nodes[128 + 63].right,
            NodeRef::new_internal(128 + 64 + 31)
        );
        assert_eq!(tree.internal_nodes[127].left, NodeRef::new_internal(63));
        assert_eq!(
            tree.internal_nodes[127].right,
            NodeRef::new_internal(128 + 31)
        );
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzig_left() {
        let mut tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0xbf));
        assert_eq!(tree.internal_nodes[0x3f].left, NodeRef::new_internal(0x1f));
        assert_eq!(tree.internal_nodes[0x3f].right, NodeRef::new_internal(0x5f));
        assert_eq!(tree.internal_nodes[0x1f].left, NodeRef::new_internal(0x0f));
        assert_eq!(tree.internal_nodes[0x1f].right, NodeRef::new_internal(0x2f));
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Left);
            walker.go(Direction::Left);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0x1f);
        assert_eq!(tree.internal_nodes[0x1f].left, NodeRef::new_internal(0x0f));
        assert_eq!(tree.internal_nodes[0x1f].right, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x3f].left, NodeRef::new_internal(0x2f));
        assert_eq!(tree.internal_nodes[0x3f].right, NodeRef::new_internal(0x7f));
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x5f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0xbf));
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzig_right() {
        let mut tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0xbf));
        assert_eq!(tree.internal_nodes[0xbf].left, NodeRef::new_internal(0x9f));
        assert_eq!(tree.internal_nodes[0xbf].right, NodeRef::new_internal(0xdf));
        assert_eq!(tree.internal_nodes[0xdf].left, NodeRef::new_internal(0xcf));
        assert_eq!(tree.internal_nodes[0xdf].right, NodeRef::new_internal(0xef));
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.go(Direction::Right);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0xdf);
        assert_eq!(tree.internal_nodes[0xdf].left, NodeRef::new_internal(0xbf));
        assert_eq!(tree.internal_nodes[0xdf].right, NodeRef::new_internal(0xef));
        assert_eq!(tree.internal_nodes[0xbf].left, NodeRef::new_internal(0x7f));
        assert_eq!(tree.internal_nodes[0xbf].right, NodeRef::new_internal(0xcf));
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0x9f));
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzag_rightleft() {
        let mut tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0xbf));
        assert_eq!(tree.internal_nodes[0xbf].left, NodeRef::new_internal(0x9f));
        assert_eq!(tree.internal_nodes[0xbf].right, NodeRef::new_internal(0xdf));
        assert_eq!(tree.internal_nodes[0x9f].left, NodeRef::new_internal(0x8f));
        assert_eq!(tree.internal_nodes[0x9f].right, NodeRef::new_internal(0xaf));
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Right);
            walker.go(Direction::Left);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0x9f);
        assert_eq!(tree.internal_nodes[0x9f].left, NodeRef::new_internal(0x7f));
        assert_eq!(tree.internal_nodes[0x9f].right, NodeRef::new_internal(0xbf));
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0x8f));
        assert_eq!(tree.internal_nodes[0xbf].left, NodeRef::new_internal(0xaf));
        assert_eq!(tree.internal_nodes[0xbf].right, NodeRef::new_internal(0xdf));
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzag_leftright() {
        let mut tree = SplayTree::new_uniform();
        assert_eq!(tree.root, 0x7f);
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0xbf));
        assert_eq!(tree.internal_nodes[0x3f].left, NodeRef::new_internal(0x1f));
        assert_eq!(tree.internal_nodes[0x3f].right, NodeRef::new_internal(0x5f));
        assert_eq!(tree.internal_nodes[0x5f].left, NodeRef::new_internal(0x4f));
        assert_eq!(tree.internal_nodes[0x5f].right, NodeRef::new_internal(0x6f));
        {
            let mut walker = tree.splayable_mut(); // [0, 255]
            walker.go(Direction::Left);
            walker.go(Direction::Right);
            walker.splay_internal();
        }
        assert_eq!(tree.root, 0x5f);
        assert_eq!(tree.internal_nodes[0x5f].left, NodeRef::new_internal(0x3f));
        assert_eq!(tree.internal_nodes[0x5f].right, NodeRef::new_internal(0x7f));
        assert_eq!(tree.internal_nodes[0x3f].left, NodeRef::new_internal(0x1f));
        assert_eq!(tree.internal_nodes[0x3f].right, NodeRef::new_internal(0x4f));
        assert_eq!(tree.internal_nodes[0x7f].left, NodeRef::new_internal(0x6f));
        assert_eq!(tree.internal_nodes[0x7f].right, NodeRef::new_internal(0xbf));
        assert!(tree.is_consistent());
    }

    #[test]
    fn test_splay_zigzig_zigzag_zig() {
        let mut tree = SplayTree::new_uniform();
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
        let mut tree = SplayTree::new_uniform();
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
        let mut tree = SplayTree::new_uniform();
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
}
