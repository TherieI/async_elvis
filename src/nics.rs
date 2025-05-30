use smoltcp::wire::EthernetAddress;
use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::simulator::{SimErr, Topology};

pub type NicId = u64;
pub type NicGroup = u64;
pub type LinkId = u64;

#[derive(PartialEq, Eq, Debug)]
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
    pub(crate) fn link(&mut self, id: LinkId) {
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
    pub(crate) fn from_slice(nics: &'a [Nic]) -> Self {
        Self { nics }
    }

    /// Returns a nic with the associated mac address, if found.
    pub fn find_mac(&self, mac: &EthernetAddress) -> Option<&Nic> {
        self.nics.iter().find(|nic| nic.mac == *mac)
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
    node: usize,
    topology: &'a mut Topology,
}

impl<'a> NicsMut<'a> {
    pub(crate) fn from_slice(node: usize, topology: &'a mut Topology) -> Self {
        // let sectioned: Vec<&mut [Nic]> = nics.chunk_by_mut(|l, r| l.group == r.group).collect();
        Self { node, topology }
    }

    /// Link with other nodes
    pub fn link(&mut self, local_id: NicId, next_hop: &EthernetAddress) -> Result<(), NicError> {
        // Ensure the nic currently is not in use

        if let Some(neighbor) = self
            .topology
            .all_nics()
            .iter()
            .find(|nic| nic.mac == *next_hop)
        {
            self.topology.link_nics(local_id, neighbor.id);
            Ok(())
        } else {
            Err(NicError::NeighborNotFound)
        }
    }
}

impl<'a> Index<usize> for NicsMut<'a> {
    type Output = Nic;

    fn index(&self, index: usize) -> &Self::Output {
        &self.topology.nics(self.node)[index]
    }
}

impl<'a> IndexMut<usize> for NicsMut<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.topology.nics_mut(self.node)[index]
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
    /// Add a nic to the node.
    ///
    /// # Panics!
    /// If the total number of nics generated in the simulation exceeds the capacity of a `u64`.
    pub fn nic(&mut self, mac: EthernetAddress, latency: Option<u64>) {
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

    /// If a node has not initialized at least one NIC.
    pub(crate) fn next_node(&mut self) -> Result<(), SimErr> {
        // Assert that at least one NIC has been initialized by the user
        // assert_eq!(self.ngroup, self.nics[self.nics.len() - 1].group);
        if self.nics.len() <= 0 || self.ngroup != self.nics[self.nics.len() - 1].group {
            return Result::Err(SimErr::NodeNoHardware);
        }
        self.ngroup = self
            .ngroup
            .checked_add(1)
            .expect("The number of nodes should be less than or equal to `u64::MAX`");
        Ok(())
    }

    pub(crate) fn to_vec(self) -> Vec<Nic> {
        self.nics
    }

    pub(crate) fn total(&self) -> usize {
        self.nics.len()
    }
}
