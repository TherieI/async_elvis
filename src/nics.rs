use smoltcp::wire::EthernetAddress;
use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::simulator::Topology;

pub type NicId = u64;
pub type NicGroup = u64;
pub type LinkId = u64;

pub struct Nic {
    pub(crate) id: NicId,
    /// The node the Nic is accociated with
    pub(crate) group: NicGroup,

    pub(crate) mac: EthernetAddress,
    pub(crate) latency: Option<u64>,

    // A link id will be generated when two nodes connect. The value will be shared across both NICs.
    pub(crate) link_id: Option<LinkId>,
}

impl Nic {
    pub fn link(&mut self, id: LinkId) {
        self.link_id = Some(id);
    }
}

#[derive(Debug)]
pub enum NicError {
    NeighborNotFound,
}

// Instead of having a Nics struct, perhaps return a slice of nics vec
pub struct Nics<'a> {
    nics: &'a [Nic],
}

impl<'a> Nics<'a> {
    pub(crate) fn from_slice(nics: &'a mut [Nic]) -> Self {
        Self { nics }
    }

    pub fn mac(&self, address: &EthernetAddress) -> Option<&Nic> {
        self.nics.iter().find(|nic| nic.mac == *address)
    }

    pub fn len(&self) -> usize {
        self.nics.len()
    }
}

impl<'a> Index<usize> for Nics<'a> {
    type Output = Nic;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nics[index]
    }
}

/// NicsMut needs to be able to see all Nics in the simulation.
pub struct NicsMut<'a> {
    subslice: usize,
    nics: Vec<&'a mut [Nic]>,
    topology: Topology,
}

impl<'a> NicsMut<'a> {
    pub(crate) fn from_slice(
        subslice: usize,
        nics: Vec<&'a mut [Nic]>,
        topology: Topology,
    ) -> Self {
        
        // let sectioned: Vec<&mut [Nic]> = nics.chunk_by_mut(|l, r| l.group == r.group).collect();
        Self {
            subslice,
            nics,
            topology,
        }
    }

    /// Link with other nodes
    pub fn link(&mut self, local: &mut Nic, next_hop: &EthernetAddress) -> Result<(), NicError> {
        if let Some(neighbor) = self
            .nics
            .iter_mut()
            .flat_map(|slice| slice.iter_mut())
            .find(|nic| nic.mac == *next_hop)
        {
            let link_id = self.topology.link_nics(local.id, neighbor.id);
            local.link(link_id);
            neighbor.link(link_id);
            Ok(())
        } else {
            Err(NicError::NeighborNotFound)
        }
    }

    pub(crate) fn reclaim(self) -> (Vec<&'a mut [Nic]>, Topology) {
        (self.nics, self.topology)
    }
}

impl<'a> Index<usize> for NicsMut<'a> {
    type Output = Nic;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nics[self.subslice][index]
    }
}

impl<'a> IndexMut<usize> for NicsMut<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.nics[self.subslice][index]
    }
}

pub struct NicAllocator {
    /// nic-id; Counter to distribute unqiue nic ids.
    nid: u64,
    /// nic-group; Binding nics to respective nodes.
    ngroup: u64,
    nics: Vec<Nic>,
}

impl NicAllocator {
    pub fn add(&mut self, mac: EthernetAddress, latency: Option<u64>) {
        let next_id = self.nid;
        self.nid = self
            .nid
            .checked_add(1)
            .expect("The number of nics should be less than or equal to `u64::MAX`");
        self.nics.push(Nic {
            id: next_id,
            group: self.ngroup,
            mac,
            latency,
            link_id: None,
        });
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            nid: 0,
            ngroup: 0,
            nics: Vec::with_capacity(capacity),
        }
    }

    /// # Panics!
    /// If a node has not initialized at least one NIC.
    pub(crate) fn next_node(&mut self) {
        // Assert that at least one NIC has been initialized by the user
        assert_eq!(self.ngroup, self.nics[self.nics.len() - 1].group);
        self.ngroup = self
            .ngroup
            .checked_add(1)
            .expect("The number of nodes should be less than or equal to `u64::MAX`");
    }

    pub(crate) fn to_vec(self) -> Vec<Nic> {
        self.nics
    }

    pub(crate) fn total(&self) -> usize {
        self.nics.len()
    }
}
