use std::fmt::Debug;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeRef<T: Clone + Copy + Debug + Eq + PartialEq> {
    Internal(T),
    Leaf(T),
}

impl<T: Clone + Copy + Debug + Eq + PartialEq> NodeRef<T> {
    pub fn new_leaf(v: T) -> Self {
        NodeRef::Leaf(v)
    }

    pub fn new_internal(v: T, max: T) -> Self {
        assert!(v != max, "too large internal ID: {v:?}");
        NodeRef::Internal(v)
    }

    pub fn as_leaf(&self) -> Option<T> {
        match self {
            NodeRef::Leaf(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_internal(&self) -> Option<T> {
        match self {
            NodeRef::Internal(v) => Some(*v),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    pub fn from_bit(bit: bool) -> Direction {
        if bit {
            Direction::Right
        } else {
            Direction::Left
        }
    }

    #[allow(dead_code)]
    pub fn to_bit(self) -> bool {
        self == Direction::Right
    }
}

#[derive(Debug)]
pub struct Node<T: Clone + Copy + Debug + Eq + PartialEq> {
    pub left: NodeRef<T>,
    pub right: NodeRef<T>,
}

impl<T: Clone + Copy + Debug + Eq + PartialEq> Node<T> {
    pub fn arm(&self, dir: Direction) -> NodeRef<T> {
        match dir {
            Direction::Left => self.left,
            Direction::Right => self.right,
        }
    }

    pub fn arm_mut(&mut self, dir: Direction) -> &mut NodeRef<T> {
        match dir {
            Direction::Left => &mut self.left,
            Direction::Right => &mut self.right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_sizes_u8() {
        // "1 + padding" = size_of::<u8>(), meh
        assert_eq!(size_of::<u8>() + size_of::<u8>(), size_of::<NodeRef<u8>>());
    }

    #[test]
    fn test_ref_leaf_u8() {
        let leaf_0 = NodeRef::<u8>::new_leaf(0);
        let leaf_10 = NodeRef::<u8>::new_leaf(10);
        let leaf_max = NodeRef::<u8>::new_leaf(u8::MAX);
        assert_eq!(leaf_0.as_leaf(), Some(0));
        assert_eq!(leaf_10.as_leaf(), Some(10));
        assert_eq!(leaf_max.as_leaf(), Some(u8::MAX));
        assert_eq!(leaf_0.as_internal(), None);
        assert_eq!(leaf_10.as_internal(), None);
        assert_eq!(leaf_max.as_internal(), None);
    }

    #[test]
    fn test_ref_internal_u8() {
        let int_0 = NodeRef::<u8>::new_internal(0, u8::MAX);
        let int_10 = NodeRef::<u8>::new_internal(10, u8::MAX);
        let int_max = NodeRef::<u8>::new_internal(u8::MAX - 1, u8::MAX);
        assert_eq!(int_0.as_internal(), Some(0));
        assert_eq!(int_10.as_internal(), Some(10));
        assert_eq!(int_max.as_internal(), Some(u8::MAX - 1));
        assert_eq!(int_0.as_leaf(), None);
        assert_eq!(int_10.as_leaf(), None);
        assert_eq!(int_max.as_leaf(), None);
    }

    #[test]
    #[should_panic = "too large internal ID"]
    fn test_ref_internal_overflow_u8() {
        NodeRef::<u8>::new_internal(u8::MAX, u8::MAX);
    }

    #[test]
    fn test_sizes_u16() {
        // "1 + padding" = size_of::<u16>(), meh
        assert_eq!(
            size_of::<u16>() + size_of::<u8>() + 1,
            size_of::<NodeRef<u16>>()
        );
    }

    #[test]
    fn test_ref_leaf_u16() {
        let leaf_0 = NodeRef::<u16>::new_leaf(0);
        let leaf_10 = NodeRef::<u16>::new_leaf(10);
        let leaf_999 = NodeRef::<u16>::new_leaf(999);
        let leaf_max = NodeRef::<u16>::new_leaf(u16::MAX);
        assert_eq!(leaf_0.as_leaf(), Some(0));
        assert_eq!(leaf_10.as_leaf(), Some(10));
        assert_eq!(leaf_999.as_leaf(), Some(999));
        assert_eq!(leaf_max.as_leaf(), Some(u16::MAX));
        assert_eq!(leaf_0.as_internal(), None);
        assert_eq!(leaf_10.as_internal(), None);
        assert_eq!(leaf_max.as_internal(), None);
    }

    #[test]
    fn test_ref_internal_u16() {
        let int_0 = NodeRef::<u16>::new_internal(0, u16::MAX);
        let int_10 = NodeRef::<u16>::new_internal(10, u16::MAX);
        let int_999 = NodeRef::<u16>::new_internal(999, u16::MAX);
        let int_max = NodeRef::<u16>::new_internal(u16::MAX - 1, u16::MAX);
        assert_eq!(int_0.as_internal(), Some(0));
        assert_eq!(int_10.as_internal(), Some(10));
        assert_eq!(int_999.as_internal(), Some(999));
        assert_eq!(int_max.as_internal(), Some(u16::MAX - 1));
        assert_eq!(int_0.as_leaf(), None);
        assert_eq!(int_10.as_leaf(), None);
        assert_eq!(int_999.as_leaf(), None);
        assert_eq!(int_max.as_leaf(), None);
    }

    #[test]
    #[should_panic = "too large internal ID"]
    fn test_ref_internal_overflow_u16() {
        NodeRef::<u16>::new_internal(u16::MAX, u16::MAX);
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
