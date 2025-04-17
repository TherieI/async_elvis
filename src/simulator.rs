use std::collections::HashMap;

use crate::nics::{LinkId, Nic, NicId, Nics, NicsMut};
use crate::{nics::NicAllocator, node::Node};

pub(crate) struct Topology {
    next_id: LinkId,
    links: HashMap<LinkId, (NicId, NicId)>,
}

impl Topology {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            next_id: 0,
            links: HashMap::with_capacity(capacity),
        }
    }

    pub fn link_nics(&mut self, nic1: NicId, nic2: NicId) -> LinkId {
        let id = self.next_id;
        self.links.insert(id, (nic1, nic2));
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("The number of links should be less than or equal to `u64::MAX`");
        id
    }
}

fn run_sim(nodes: &mut [&mut dyn Node]) {
    // Note: This array must not change once the nics have been generated.
    let mut topology = Topology::with_capacity(nodes.len());

    let mut nic_allocator = NicAllocator::with_capacity(nodes.len());
    // Generate the hardware for each node.
    for node in nodes.iter_mut() {
        node.hardware(&mut nic_allocator);
        nic_allocator.next_node();
    }
    // nics should never change size after this point
    let mut nics = nic_allocator.to_vec();

    let mut hardware: Vec<&mut [Nic]> = nics.chunk_by_mut(|l, r| l.group == r.group).collect();
    assert_eq!(nodes.len(), hardware.len());

    // Run startup for each node.
    for (i, node) in nodes.iter_mut().enumerate() {
        // The rust compiler hates it when hardware and topology are mutable references, so NicsMut have to own them.
        let mut nics_mut = NicsMut::from_slice(i, hardware, topology);
        node.startup(&mut nics_mut);
        (hardware, topology) = nics_mut.reclaim();
    }
}
