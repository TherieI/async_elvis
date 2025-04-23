use std::collections::HashMap;

use crate::nics::{LinkId, Nic, NicId, Nics, NicsMut};
use crate::{nics::NicAllocator, node::Node};

/// Calculates the bounds for a slice of nics that correspond with a node.
fn slice_bounds(nics: &[Nic], node: usize) -> Option<(usize, usize)> {
    // Find slice range
    let start = nics.iter().position(|nic| nic.group == node as u64)?;
    let end = start
        + nics[start..]
            .iter()
            .take_while(|nic| nic.group == node as u64)
            .count();
    Some((start, end))
}

pub(crate) struct Topology {
    next_id: LinkId,
    hardware: Vec<Nic>,
    // Links are full-duplex
    links: HashMap<LinkId, (NicId, NicId)>,
}

impl Topology {
    fn new(hardware: Vec<Nic>, capacity: usize) -> Self {
        Self {
            next_id: 0,
            hardware,
            links: HashMap::with_capacity(capacity),
        }
    }

    /// Return an immutable slice over the nics of a node.
    pub(crate) fn nics(&self, node: usize) -> &[Nic] {
        let (start, end) =
            slice_bounds(&self.hardware[node..], node).expect("node should be within bounds");
        &self.hardware[start..end]
    }

    /// Return a mutable slice over the nics of a node.
    pub(crate) fn nics_mut(&mut self, node: usize) -> &mut [Nic] {
        let (start, end) =
            slice_bounds(&self.hardware[node..], node).expect("node should be within bounds");
        &mut self.hardware[start..end]
    }

    // pub(crate) fn node_slice(&mut self) -> Vec<&mut [Nic]> {
    //     self.hardware
    //         .chunk_by_mut(|l, r| l.group == r.group)
    //         .collect()
    // }

    pub(crate) fn all_nics(&self) -> &[Nic] {
        &self.hardware
    }

    pub(crate) fn link_nics(&mut self, nic1: NicId, nic2: NicId) -> LinkId {
        let id = self.next_id;
        self.links.insert(id, (nic1, nic2));
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("The number of links should be less than or equal to `u64::MAX`");
        id
    }

    /// Call after the links hashmap is complete. This will initialize the `Option<LinkId>` field in each
    /// relevent `Nic` with the proper link.
    pub(crate) fn fill_links(&mut self) {
        for (link, nics) in self.links.iter_mut() {
            self.hardware[nics.0 as usize].link(*link);
            self.hardware[nics.1 as usize].link(*link);
        }
    }
}

fn run_sim(nodes: &mut [&mut dyn Node]) {
    // Note: This array must not change once the nics have been generated.
    let mut nic_allocator = NicAllocator::with_capacity(nodes.len());
    // Generate the hardware for each node.
    for node in nodes.iter_mut() {
        node.hardware(&mut nic_allocator);
        nic_allocator.next_node();
    }
    // nics should never change size after this point
    let mut nics = nic_allocator.to_vec();

    {
        // Assert the user has initialized at LEAST one nic per node.
        let hardware: Vec<&mut [Nic]> = nics.chunk_by_mut(|l, r| l.group == r.group).collect();
        assert_eq!(nodes.len(), hardware.len());
    }

    let mut topology = Topology::new(nics, nodes.len());

    // Run startup for each node.
    for (i, node) in nodes.iter_mut().enumerate() {
        // The rust compiler hates it when hardware and topology are mutable references, so NicsMut have to own them.
        let mut nics_mut = NicsMut::from_slice(i, &mut topology);
        node.startup(&mut nics_mut);
    }
    topology.fill_links();


}

#[cfg(test)]
mod tests {
    use super::*;
    use smoltcp::wire::EthernetAddress;

    fn nic_with_group(group: u64) -> Nic {
        Nic {
            id: 0,
            group,
            mac: EthernetAddress([0, 0, 0, 0, 0, 0]),
            latency: None,
            link_id: None,
        }
    }

    #[test]
    fn slice_bounds_check() {
        let mut nics = Vec::new();
        for i in 0..10 {
            for _ in 0..2 {
                nics.push(nic_with_group(i));
            }
        }

        // CHECK AN EVEN LIST
        let (start, end) = slice_bounds(&nics, 0).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[0..2]);

        let (start, end) = slice_bounds(&nics, 4).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[8..10]);

        let (start, end) = slice_bounds(&nics, 9).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[18..20]);

        assert!(slice_bounds(&nics, 10).is_none());

        nics.clear();

        // CHECK AN UNEVEN LIST
        nics.push(nic_with_group(0));
        nics.push(nic_with_group(0));
        nics.push(nic_with_group(1));
        nics.push(nic_with_group(2));
        nics.push(nic_with_group(2));
        nics.push(nic_with_group(2));
        nics.push(nic_with_group(3));
        nics.push(nic_with_group(4));
        nics.push(nic_with_group(4));
        nics.push(nic_with_group(5));
        nics.push(nic_with_group(5));
        nics.push(nic_with_group(5));
        nics.push(nic_with_group(6));
        nics.push(nic_with_group(7));
        nics.push(nic_with_group(8));
        nics.push(nic_with_group(8));
        nics.push(nic_with_group(8));
        nics.push(nic_with_group(8));
        nics.push(nic_with_group(8));
        nics.push(nic_with_group(9));

        let (start, end) = slice_bounds(&nics, 0).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[0..2]);

        let (start, end) = slice_bounds(&nics, 1).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[2..3]);

        let (start, end) = slice_bounds(&nics, 2).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[3..6]);

        let (start, end) = slice_bounds(&nics, 8).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[14..19]);

        let (start, end) = slice_bounds(&nics, 9).expect("Slice should be found");
        assert_eq!(&nics[start..end], &nics[19..20]);
    }
}
