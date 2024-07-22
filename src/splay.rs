use std::array::from_fn;

type BaseUnit = u8;

// Causes stack overflow by allocating 262 KiB?!
// type BaseUnit = u16;

#[derive(Debug)]
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

    fn is_subtree_consistent(&self, root_index: BaseUnit, cover_min: BaseUnit, cover_max_incl: BaseUnit) -> bool {
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

    fn is_arm_consistent(&self, root: &NodeRef, cover_min: BaseUnit, cover_max_incl: BaseUnit) -> bool {
        if let Some(child_index) = root.as_internal() {
            return self.is_subtree_consistent(child_index, cover_min, cover_max_incl);
        }
        if let Some(leaf_index) = root.as_leaf() {
            return cover_min == leaf_index && leaf_index == cover_max_incl;
        }
        panic!("empty child?!")
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
}
