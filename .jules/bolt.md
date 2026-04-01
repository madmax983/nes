## 2025-03-26 - [BTreeMap pop_first optimization]
**Learning:** Removing the smallest key/entry from a `BTreeMap` using `keys().next().copied()` followed by a `remove(&key)` is inefficient because it requires two O(log N) tree traversals (one to find the first element, one to traverse down to remove it by key).
**Action:** Always use the `BTreeMap::pop_first()` method to perform this action in a single step, saving a lookup and avoiding the `.copied()` allocation.
