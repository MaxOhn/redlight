use std::collections::HashSet;

use twilight_model::id::Id;

pub fn convert_ids_set<T>(mut ids: HashSet<u64>) -> HashSet<Id<T>> {
    ids.remove(&0);

    // SAFETY: we ensured that all u64s are non-zero
    unsafe { std::mem::transmute(ids) }
}

pub fn convert_ids_vec<T>(mut ids: Vec<u64>) -> Vec<Id<T>> {
    ids.retain(|&id| id != 0);

    // SAFETY: we ensured that all u64s are non-zero
    unsafe { std::mem::transmute(ids) }
}
