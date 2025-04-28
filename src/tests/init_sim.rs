use smoltcp::wire::EthernetAddress;

use crate::simulator::{run_sim, sim_setup};
use crate::{
    nics::{NicAllocator, Nics, NicsMut},
    node::{Mailbox, Node, NodeError},
    nodes,
};

const ETH0: EthernetAddress = EthernetAddress([0, 0, 0, 0, 0, 0]);
const ETH1: EthernetAddress = EthernetAddress([0, 0, 0, 0, 0, 1]);
const ETH2: EthernetAddress = EthernetAddress([0, 0, 0, 0, 0, 2]);
const ETH3: EthernetAddress = EthernetAddress([0, 0, 0, 0, 0, 3]);
const ETH4: EthernetAddress = EthernetAddress([0, 0, 0, 0, 0, 4]);
const ETH5: EthernetAddress = EthernetAddress([0, 0, 0, 0, 0, 5]);

struct BasicNode {
    eth: Option<EthernetAddress>,
    neighbor: Option<EthernetAddress>,
}

impl BasicNode {
    fn broken() -> Self {
        Self {
            eth: None,
            neighbor: None,
        }
    }

    fn new(addr: EthernetAddress) -> Self {
        Self {
            eth: Some(addr),
            neighbor: None,
        }
    }

    fn set_neighbor(mut self, neighbor: EthernetAddress) -> Self {
        self.neighbor = Some(neighbor);
        self
    }
}

#[async_trait::async_trait]
impl Node for BasicNode {
    fn hardware(&self, nics: &mut NicAllocator) {
        if let Some(addr) = self.eth {
            nics.nic(addr, None);
        }
    }

    fn startup(&mut self, nics: &mut NicsMut<'_>) {
        if let Some(addr) = &self.neighbor {
            nics.link(nics[0].id, addr).unwrap();
        }
    }

    async fn process(&mut self, _: &mut Mailbox, _: &Nics<'_>) -> Result<(), NodeError> {
        Ok(())
    }
}

#[test]
fn sim_setup_failure_cases() {
    run_sim(nodes![BasicNode::broken()]).expect_err("Node should have at least one NIC");

    run_sim(nodes![BasicNode::new(ETH0), BasicNode::broken()])
        .expect_err("Node should have at least one NIC");
}

#[test]
fn sim_setup_success_cases() {
    let _ = sim_setup(nodes![
        BasicNode::new(ETH0),
        BasicNode::new(ETH1),
        BasicNode::new(ETH2)
    ])
    .expect("Sim correctly initializes");

    let links = sim_setup(nodes![
        BasicNode::new(ETH0).set_neighbor(ETH1),
        BasicNode::new(ETH1),
        BasicNode::new(ETH2).set_neighbor(ETH3),
        BasicNode::new(ETH3)
    ])
    .expect("Sim correctly initializes");
    assert_eq!(links.links.len(), 2);

    let mut iter = links.links.iter();
    assert_eq!(iter.next().unwrap(), &(0u64, 1u64));
    assert_eq!(iter.next().unwrap(), &(2u64, 3u64));
}
