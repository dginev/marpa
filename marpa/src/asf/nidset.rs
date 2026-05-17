#[derive(Debug, Clone)]
pub struct Nidset {
    pub(crate) nids: Vec<i32>,
    // `id` is the nidset's identity (= the glade's `id` field for
    // glades that wrap exactly this nidset). Read indirectly via
    // hash lookups; mark allow(dead_code) since direct reads are
    // rare.
    #[allow(dead_code)]
    pub(crate) id: usize,
}

impl Nidset {
    pub fn get_nid(&self, index: usize) -> i32 {
        self.nids[index]
    }
}
